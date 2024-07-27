#include <efi.h>
#include <efilib.h>
#include <elf.h>

// Define constants for the target screen width, height, and pixel format.
#define TARGET_SCREEN_WIDTH 1360
#define TARGET_SCREEN_HEIGHT 768
#define TARGET_PIXEL_FORMAT PixelBlueGreenRedReserved8BitPerColor

// Framebuffer structure representing a basic framebuffer.
typedef struct
{
    VOID *pointer;              // Pointer to the beginning of the framebuffer in memory.
    UINT32 width;               // Width of the framebuffer in pixels.
    UINT32 height;              // Height of the framebuffer in pixels.
    UINT32 pixels_per_scanline; // Number of pixels per scanline (often equals width but can be larger).
} Framebuffer;

// PSF1 font header structure.
typedef struct
{
    unsigned char magic[2];  // Magic number identifying the PSF1 format.
    unsigned char mode;      // PSF1 mode (0 or 1).
    unsigned char char_size; // Size of each character in bytes.
} PSF1_HEADER;

// PSF1 font structure containing the font header and glyph buffer.
typedef struct
{
    PSF1_HEADER psf1_header; // Pointer to the PSF1 font header.
    void *glyph_buffer;      // Pointer to the glyph buffer containing the font's glyphs.
} PSF1_FONT;

// Boot_Info structure containing information passed to the OS at boot time.
typedef struct
{
    EFI_MEMORY_DESCRIPTOR *memory_map; // Pointer to the system's memory map.
    UINTN memory_map_size;             // Total size of the memory map.
    UINTN memory_map_descriptor_size;  // Size of an individual memory descriptor in the memory map.
    Framebuffer framebuffer;           // Framebuffer information for the display.
    PSF1_FONT font;                    // Pointer to the loaded PSF1 font.
    UINT64 kernel_end;                 // The end address of the kernel.
    void *rsdp;                        // Pointer to the RSDP ACPI table.
} Boot_Info;

// Global variable to hold the handle buffer for graphics protocols.
EFI_HANDLE *graphic_handle_buffer;

// Global variable to hold the file system protocol interface.
EFI_SIMPLE_FILE_SYSTEM_PROTOCOL *fs_protocol;

// Global variable to hold the kernel end address.
UINT64 kernel_end;

// Ensure you have the correct GUID for ACPI 2.0 Table
EFI_GUID Acpi2TableGuid = ACPI_20_TABLE_GUID;

// Simple implementation of memcmp using EFI types
INTN compareMemory(CONST VOID *dstBuffer, CONST VOID *srcBuffer, UINTN n)
{
    CONST UINT8 *dest = dstBuffer;
    CONST UINT8 *src = srcBuffer;

    for (UINTN i = 0; i < n; i++)
    {
        if (dest[i] != src[i])
        {
            return dest[i] - src[i];
        }
    }
    return 0;
}

/**
 * load_segment - Loads a segment of a kernel image into memory.
 *
 * This function is responsible for loading a specified segment of a kernel image
 * from a file into a designated area in memory. It involves setting the file position,
 * allocating memory for the segment, reading the segment data from the file, and
 * copying this data into the allocated memory. If the segment in memory is larger
 * than the segment in the file, the extra memory is zero-filled.
 *
 * Parameters:
 *   kernel_img_file - Pointer to the EFI_FILE structure representing the kernel image file.
 *   segment_file_offset - The offset in the file where the segment data begins.
 *   segment_file_size - The size of the segment in the file.
 *   segment_memory_size - The size of the memory area where the segment is to be loaded.
 *   segment_physical_address - The physical address in memory where the segment will be loaded.
 *
 * Returns:
 *   EFI_STATUS - The status of the operation. EFI_SUCCESS if the operation was successful,
 *                or an appropriate error code if there was an error.
 */
EFI_STATUS load_segment(IN EFI_FILE *const kernel_img_file,
                        IN EFI_PHYSICAL_ADDRESS const segment_file_offset,
                        IN UINTN const segment_file_size,
                        IN UINTN const segment_memory_size,
                        IN EFI_PHYSICAL_ADDRESS const segment_physical_address)
{
    EFI_STATUS status;
    VOID *program_data = NULL;                                         // Buffer to read segment data into
    UINTN buffer_read_size = 0;                                        // Size of segment data read
    UINTN segment_page_count = EFI_SIZE_TO_PAGES(segment_memory_size); // Number of pages to allocate for segment
    EFI_PHYSICAL_ADDRESS zero_fill_start = 0;                          // Start address of zero fill
    UINTN zero_fill_count = 0;                                         // Number of bytes to zero fill

    Print(L"Setting file pointer to segment offset '0x%llx'\n", segment_file_offset);
    status = uefi_call_wrapper(kernel_img_file->SetPosition, 2, kernel_img_file, segment_file_offset);
    if (EFI_ERROR(status))
    {
        Print(L"Error: Error setting file pointer to segment offset: %r\n", status);
        return status;
    }

    Print(L"Allocating %lu pages at address '0x%llx'\n", segment_page_count, segment_physical_address);
    status = uefi_call_wrapper(gBS->AllocatePages, 4, AllocateAddress, EfiLoaderData, segment_page_count, (EFI_PHYSICAL_ADDRESS *)&segment_physical_address);
    if (EFI_ERROR(status))
    {
        Print(L"Error: Error allocating pages for segment: %r\n", status);
        return status;
    }

    // If the segment has a file size greater than 0, read the segment data into memory.
    if (segment_file_size > 0)
    {
        buffer_read_size = segment_file_size; // Set buffer read size to segment file size
        Print(L"Allocating buffer for segment data of size '0x%lx'\n", buffer_read_size);
        status = uefi_call_wrapper(gBS->AllocatePool, 3, EfiLoaderCode, buffer_read_size, (VOID **)&program_data);
        if (EFI_ERROR(status))
        {
            Print(L"Error: Error allocating buffer for segment data: %r\n", status);
            return status;
        }

        Print(L"Reading segment data\n");
        status = uefi_call_wrapper(kernel_img_file->Read, 3, kernel_img_file, &buffer_read_size, (VOID *)program_data);
        if (EFI_ERROR(status))
        {
            Print(L"Error: Error reading segment data: %r\n", status);
            return status;
        }

        Print(L"Copying segment to memory address '0x%llx'\n", segment_physical_address);
        status = uefi_call_wrapper(gBS->CopyMem, 3, segment_physical_address, program_data, segment_file_size);
        if (EFI_ERROR(status))
        {
            Print(L"Error: Error copying segment to memory: %r\n", status);
            return status;
        }

        Print(L"Freeing program data buffer\n");
        status = uefi_call_wrapper(gBS->FreePool, 1, program_data);
        if (EFI_ERROR(status))
        {
            Print(L"Error: Error freeing program data buffer: %r\n", status);
            return status;
        }
    }

    // If the segment memory size is greater than the segment file size, zero fill the remaining memory.
    zero_fill_start = segment_physical_address + segment_file_size;
    zero_fill_count = segment_memory_size - segment_file_size;
    if (zero_fill_count > 0)
    {
        Print(L"Zero filling segment from '0x%llx' to '0x%llx'\n", zero_fill_start, zero_fill_start + zero_fill_count);
        status = uefi_call_wrapper(gBS->SetMem, 3, zero_fill_start, zero_fill_count, 0);
        if (EFI_ERROR(status))
        {
            Print(L"Error: Error zero filling segment: %r\n", status);
            return status;
        }
    }

    return EFI_SUCCESS;
}

/**
 * load_program_segments - Loads all loadable segments of a kernel image into memory.
 *
 * This function iterates through the program headers of a kernel image and loads each
 * segment marked as loadable (PT_LOAD) into memory. It utilizes the 'load_segment' function
 * to perform the actual loading of individual segments.
 *
 * Parameters:
 *   kernel_img_file - Pointer to the EFI_FILE structure representing the kernel image file.
 *   kernel_header_buffer - Pointer to a buffer containing the kernel's main header (Elf64_Ehdr).
 *   kernel_program_headers_buffer - Pointer to a buffer containing the kernel's program headers (Elf64_Phdr).
 *
 * Returns:
 *   EFI_STATUS - The status of the operation. Returns EFI_SUCCESS if all loadable segments are
 *                successfully loaded. Returns specific error codes in case of failures.
 */
EFI_STATUS load_program_segments(IN EFI_FILE *const kernel_img_file,
                                 IN VOID *const kernel_header_buffer,
                                 IN VOID *const kernel_program_headers_buffer)
{
    EFI_STATUS status;
    UINT16 n_program_headers = 0; // Number of program headers
    UINT16 n_segments_loaded = 0; // Number of segments loaded
    UINTN p = 0;                  // Program header index

    // Get the number of program headers from the kernel header.
    n_program_headers = ((Elf64_Ehdr *)kernel_header_buffer)->e_phnum;

    // If there are no program headers, return an error.
    if (n_program_headers == 0)
    {
        Print(L"Error: No program headers found in Kernel image\n");
        return EFI_INVALID_PARAMETER;
    }

    Print(L"Loading %u segments\n", n_program_headers);

    // Pointer to the array of program headers.
    Elf64_Phdr *program_headers = (Elf64_Phdr *)kernel_program_headers_buffer;

    // Iterate through the program headers and load each loadable segment.
    for (p = 0; p < n_program_headers; p++)
    {
        // If the program header type is PT_LOAD, load the segment.
        if (program_headers[p].p_type == PT_LOAD)
        {
            // Load the segment into memory.
            status = load_segment(kernel_img_file,
                                  program_headers[p].p_offset,
                                  program_headers[p].p_filesz,
                                  program_headers[p].p_memsz,
                                  program_headers[p].p_paddr);
            if (EFI_ERROR(status))
            {
                return status;
            }

            n_segments_loaded++;
        }
    }

    // If no loadable segments were found, return an error.
    if (n_segments_loaded == 0)
    {
        Print(L"Error: No loadable segments found in Kernel image\n");
        return EFI_NOT_FOUND;
    }

    return EFI_SUCCESS;
}

/**
 * read_elf_identity - Reads the ELF identity of a kernel image.
 *
 * This function reads the initial bytes of an ELF-formatted kernel image file to extract its identity.
 * The ELF identity is a sequence of bytes at the start of the file that contains crucial information
 * about the file format, such as the magic number and architecture type.
 *
 * Parameters:
 *   kernel_img_file - Pointer to the EFI_FILE structure representing the kernel image file.
 *   elf_identity_buffer - Pointer to a buffer that will hold the ELF identity after reading.
 *
 * Returns:
 *   EFI_STATUS - The status of the operation. Returns EFI_SUCCESS if the ELF identity is
 *                successfully read. Returns specific error codes in case of failures.
 */
EFI_STATUS read_elf_identity(IN EFI_FILE *const kernel_img_file,
                             OUT UINT8 **elf_identity_buffer)
{
    Print(L"Reading ELF identity\n");

    UINTN buffer_read_size = EI_NIDENT; // Size of ELF identity buffer
    EFI_STATUS status;

    // Set the file pointer to the start of the kernel image.
    status = uefi_call_wrapper(kernel_img_file->SetPosition, 2, kernel_img_file, 0);
    if (EFI_ERROR(status))
    {
        Print(L"Error: Error setting file pointer to start of kernel image: %r\n", status);
        return status;
    }

    // Allocate memory for the ELF identity buffer.
    status = uefi_call_wrapper(gBS->AllocatePool, 3, EfiLoaderData, EI_NIDENT, (VOID **)elf_identity_buffer);
    if (EFI_ERROR(status))
    {
        Print(L"Error: Error allocating memory for kernel identity buffer: %r\n", status);
        return status;
    }

    // Read the ELF identity from the kernel image file.
    status = uefi_call_wrapper(kernel_img_file->Read, 3, kernel_img_file, &buffer_read_size, (VOID *)*elf_identity_buffer);
    if (EFI_ERROR(status))
    {
        Print(L"Error: Error reading kernel identity buffer: %r\n", status);
        return status;
    }

    Print(L"Read ELF identity\n");

    return EFI_SUCCESS;
}

/**
 * read_elf_file - Reads the header and program headers of an ELF-formatted kernel image.
 *
 * This function reads the main header (Elf64_Ehdr) and the program headers (Elf64_Phdr)
 * of an ELF-formatted kernel image file. It allocates memory for and populates buffers
 * with these headers for further processing.
 *
 * Parameters:
 *   kernel_img_file - Pointer to the EFI_FILE structure representing the kernel image file.
 *   kernel_header_buffer - Pointer to a buffer that will hold the kernel's main header (Elf64_Ehdr) after reading.
 *   kernel_program_headers_buffer - Pointer to a buffer that will hold the kernel's program headers (Elf64_Phdr) after reading.
 *
 * Returns:
 *   EFI_STATUS - The status of the operation. Returns EFI_SUCCESS if both the kernel header and
 *                program headers are successfully read. Returns specific error codes in case of failures.
 */
EFI_STATUS read_elf_file(IN EFI_FILE *const kernel_img_file,
                         OUT VOID **kernel_header_buffer,
                         OUT VOID **kernel_program_headers_buffer)
{
    UINTN program_headers_offset = 0; // Offset of program headers in the file

    Print(L"Reading ELF file\n");

    // Set the file pointer to the start of the kernel image.
    EFI_STATUS status = uefi_call_wrapper(kernel_img_file->SetPosition, 2, kernel_img_file, 0);
    if (EFI_ERROR(status))
    {
        Print(L"Error: Error setting file pointer to start of kernel image: %r\n", status);
        return status;
    }

    UINTN buffer_read_size = sizeof(Elf64_Ehdr); // Size of kernel header buffer

    Print(L"Allocating memory for kernel header buffer\n");
    status = uefi_call_wrapper(gBS->AllocatePool, 3, EfiLoaderData, buffer_read_size, kernel_header_buffer);
    if (EFI_ERROR(status))
    {
        Print(L"Error: Error allocating kernel header buffer: %r\n", status);
        return status;
    }

    Print(L"Reading kernel executable header\n");
    status = uefi_call_wrapper(kernel_img_file->Read, 3, kernel_img_file, &buffer_read_size, *kernel_header_buffer);
    if (EFI_ERROR(status))
    {
        Print(L"Error: Error reading kernel header: %r\n", status);
        return status;
    }

    program_headers_offset = ((Elf64_Ehdr *)*kernel_header_buffer)->e_phoff;                // Get program headers offset
    buffer_read_size = sizeof(Elf64_Phdr) * ((Elf64_Ehdr *)*kernel_header_buffer)->e_phnum; // Size of program headers buffer

    Print(L"Setting file pointer to program headers offset\n");
    status = uefi_call_wrapper(kernel_img_file->SetPosition, 2, kernel_img_file, program_headers_offset);
    if (EFI_ERROR(status))
    {
        Print(L"Error: Error setting file pointer to program headers offset: %r\n", status);
        return status;
    }

    Print(L"Allocating memory for kernel program header buffer\n");
    status = uefi_call_wrapper(gBS->AllocatePool, 3, EfiLoaderData, buffer_read_size, kernel_program_headers_buffer);
    if (EFI_ERROR(status))
    {
        Print(L"Error: Error allocating kernel program headers buffer: %r\n", status);
        return status;
    }

    Print(L"Reading kernel program headers\n");
    status = uefi_call_wrapper(kernel_img_file->Read, 3, kernel_img_file, &buffer_read_size, *kernel_program_headers_buffer);
    if (EFI_ERROR(status))
    {
        Print(L"Error: Error reading kernel program headers: %r\n", status);
        return status;
    }

    return EFI_SUCCESS;
}

/**
 * load_kernel_image - Loads a kernel image from a file system into memory.
 *
 * This function opens a kernel image file, reads its ELF identity, main header,
 * and program headers, then loads the loadable segments into memory. It sets the
 * kernel entry point address for further boot processing.
 *
 * Parameters:
 *   root_file_system - Pointer to the EFI_FILE structure representing the root file system.
 *   kernel_image_filename - The file name of the kernel image to load.
 *   kernel_entry_point - Pointer to an EFI_PHYSICAL_ADDRESS to store the kernel's entry point address.
 *
 * Returns:
 *   EFI_STATUS - The status of the operation. Returns EFI_SUCCESS if the kernel image is
 *                successfully loaded. Returns specific error codes in case of failures.
 */
EFI_STATUS load_kernel_image(IN EFI_FILE *const root_file_system,
                             IN CHAR16 *const kernel_image_filename,
                             OUT EFI_PHYSICAL_ADDRESS *kernel_entry_point)
{
    EFI_STATUS status;
    EFI_FILE *kernel_img_file;           // Pointer to the kernel image file
    VOID *kernel_header = NULL;          // Buffer to hold the kernel header
    VOID *kernel_program_headers = NULL; // Buffer to hold the kernel program headers
    UINT8 *elf_identity_buffer = NULL;   // Buffer to hold the ELF identity

    Print(L"Reading kernel image\n");

    // Open the kernel image file.
    status = uefi_call_wrapper(root_file_system->Open, 5, root_file_system, &kernel_img_file, kernel_image_filename, EFI_FILE_MODE_READ, EFI_FILE_READ_ONLY);
    if (EFI_ERROR(status))
    {
        Print(L"Error: Failed to open kernel image: %r\n", status);
        return status;
    }

    // Read the ELF identity of the kernel image.
    status = read_elf_identity(kernel_img_file, &elf_identity_buffer);
    if (EFI_ERROR(status))
    {
        // Error message printed in validation function.
        return status;
    }

    // Free the ELF identity buffer.
    status = uefi_call_wrapper(gBS->FreePool, 1, elf_identity_buffer);
    if (EFI_ERROR(status))
    {
        Print(L"Error: Failed to free ELF identity buffer: %r\n", status);
        return status;
    }

    // Read the ELF file and program headers.
    status = read_elf_file(kernel_img_file, &kernel_header, &kernel_program_headers);
    if (EFI_ERROR(status))
    {
        // Error message printed in read function.
        return status;
    }

    // Set the kernel entry point address.
    *kernel_entry_point = ((Elf64_Ehdr *)kernel_header)->e_entry;

    Elf64_Ehdr *ehdr = (Elf64_Ehdr *)kernel_header;
    Elf64_Phdr *phdrs = (Elf64_Phdr *)kernel_program_headers;

    for (int i = 0; i < ehdr->e_phnum; i++)
    {
        Elf64_Phdr *phdr = &phdrs[i];
        UINT64 segment_end = phdr->p_vaddr + phdr->p_memsz;
        if (segment_end > kernel_end)
        {
            kernel_end = segment_end;
        }
    }

    // Load the program segments into memory.
    status = load_program_segments(kernel_img_file, kernel_header, kernel_program_headers);
    if (EFI_ERROR(status))
    {
        // Error message printed in load function.
        return status;
    }

    Print(L"Closing kernel image\n");
    status = uefi_call_wrapper(kernel_img_file->Close, 1, kernel_img_file);
    if (EFI_ERROR(status))
    {
        Print(L"Error: Failed to close kernel image: %r\n", status);
        return status;
    }

    Print(L"Freeing kernel header buffer\n");
    status = uefi_call_wrapper(gBS->FreePool, 1, (VOID *)kernel_header);
    if (EFI_ERROR(status))
    {
        Print(L"Error: Failed to free kernel header buffer: %r\n", status);
        return status;
    }

    Print(L"Freeing kernel program headers buffer\n");
    status = uefi_call_wrapper(gBS->FreePool, 1, (VOID *)kernel_program_headers);
    if (EFI_ERROR(status))
    {
        Print(L"Error: Failed to free kernel program headers buffer: %r\n", status);
        return status;
    }

    return status;
}

/**
 * get_memory_map - Retrieves the system's memory map.
 *
 * This function allocates memory for and retrieves the UEFI memory map,
 * which is a crucial step in the boot process. It ensures that the OS
 * has information about the memory layout of the system.
 *
 * Parameters:
 *   memory_map - A pointer to where the memory map pointer will be stored.
 *   memory_map_size - A pointer to where the size of the memory map will be stored.
 *   memory_map_key - A pointer to where the memory map key will be stored.
 *   descriptor_size - A pointer to where the size of an individual memory descriptor will be stored.
 *   descriptor_version - A pointer to where the descriptor version number will be stored.
 *
 * Returns:
 *   EFI_STATUS - The status of the operation. Returns EFI_SUCCESS if the memory map is successfully retrieved.
 *                Returns specific error codes in case of failures.
 */
EFI_STATUS get_memory_map(OUT VOID **memory_map,
                          OUT UINTN *memory_map_size,
                          OUT UINTN *memory_map_key,
                          OUT UINTN *descriptor_size,
                          OUT UINT32 *descriptor_version)
{
    EFI_STATUS status;

    Print(L"Allocating memory map\n");

    // Attempt to retrieve the memory map size.
    status = uefi_call_wrapper(gBS->GetMemoryMap, 5,
                               memory_map_size, *memory_map, memory_map_key,
                               descriptor_size, descriptor_version);
    if (EFI_ERROR(status))
    {
        // Check for buffer size inadequacy, which is common on the first call.
        if (status != EFI_BUFFER_TOO_SMALL)
        {
            Print(L"Error: Failing to get memory map: %r\n", status);
            return status;
        }
    }

    // According to: https://stackoverflow.com/a/39674958/5931673
    // Up to two new descriptors may be created in the process of allocating the
    // new pool memory.
    *memory_map_size += 2 * (*descriptor_size);

    // Allocate memory for the memory map.
    status = uefi_call_wrapper(gBS->AllocatePool, 3,
                               EfiLoaderData, *memory_map_size, (VOID **)memory_map);
    if (EFI_ERROR(status))
    {
        Print(L"Error: Failed to allocate memory for memory map: %r\n", status);
        return status;
    }

    // Retrieve the memory map.
    status = uefi_call_wrapper(gBS->GetMemoryMap, 5,
                               memory_map_size, *memory_map, memory_map_key,
                               descriptor_size, descriptor_version);
    if (EFI_ERROR(status))
    {
        Print(L"Error: Failed to get memory map: %r\n", status);
        return status;
    }

    return EFI_SUCCESS;
}

/**
 * init_gop - Initializes the Graphics Output Protocol service.
 *
 * This function locates the handles that support the Graphics Output Protocol (GOP)
 * and stores them in a global buffer for later use. The GOP is essential for
 * setting up graphical displays in UEFI applications.
 *
 * Returns:
 *   EFI_STATUS - The status of the operation. Returns EFI_SUCCESS if the GOP handles
 *                are successfully located. Returns specific error codes in case of failures.
 */
EFI_STATUS init_gop(void)
{
    EFI_STATUS status;
    UINTN graphic_handle_count;

    Print(L"Initialising Graphics Output Service\n");

    // Locate the handles that support the Graphics Output Protocol (GOP).
    status = uefi_call_wrapper(gBS->LocateHandleBuffer, 5,
                               ByProtocol, &gEfiGraphicsOutputProtocolGuid, NULL,
                               &graphic_handle_count, &graphic_handle_buffer);
    if (EFI_ERROR(status))
    {
        Print(L"Error: Failed to locate GOP handles: %r\n", status);
        return status;
    }

    Print(L"Located '%llu' GOP handles\n", graphic_handle_count);

    return EFI_SUCCESS;
}

/**
 * find_video_mode - Finds a specific video mode supported by the graphics output protocol.
 *
 * This function iterates through the available video modes supported by the provided
 * graphics output protocol instance. It looks for a mode that matches the specified
 * target width, height, and pixel format. If such a mode is found, its index is returned.
 *
 * Parameters:
 *   protocol - Pointer to the EFI_GRAPHICS_OUTPUT_PROTOCOL instance.
 *   target_width - The desired width of the video mode.
 *   target_height - The desired height of the video mode.
 *   target_pixel_format - The desired pixel format (color format) of the video mode.
 *   video_mode - Pointer to a variable where the index of the found video mode will be stored.
 *
 * Returns:
 *   EFI_STATUS - The status of the operation. Returns EFI_SUCCESS if a matching video mode is found.
 *                Returns EFI_UNSUPPORTED if no matching mode is found. Returns specific error codes in case of other failures.
 */
EFI_STATUS find_video_mode(IN EFI_GRAPHICS_OUTPUT_PROTOCOL *const protocol,
                           IN const UINT32 target_width,
                           IN const UINT32 target_height,
                           IN const EFI_GRAPHICS_PIXEL_FORMAT target_pixel_format,
                           OUT UINTN *video_mode)
{
    EFI_STATUS status;
    UINTN size_of_mode_info;
    EFI_GRAPHICS_OUTPUT_MODE_INFORMATION *mode_info;

    // Iterate through the available video modes to find a match.
    for (UINTN i = 0; i < protocol->Mode->MaxMode; i++)
    {
        // Query the size of the mode information.
        status = uefi_call_wrapper(protocol->QueryMode, 4, protocol, i, &size_of_mode_info, &mode_info);
        if (EFI_ERROR(status))
        {
            Print(L"Error: Error querying video mode: %r\n", status);
            return status;
        }

        Print(L"Mode %d: %dx%d, Pixel Format: %d\n", i, mode_info->HorizontalResolution, mode_info->VerticalResolution, mode_info->PixelFormat);

        // Check if the current mode matches the target parameters.
        if (mode_info->HorizontalResolution == target_width &&
            mode_info->VerticalResolution == target_height &&
            mode_info->PixelFormat == target_pixel_format)
        {
            // If a match is found, store the index of the mode and return success.
            *video_mode = i;
            // return EFI_SUCCESS;
        }
    }

    // If no matching mode is found, return an error.
    return EFI_SUCCESS;
}

/**
 * set_graphics_mode - Sets the graphics mode to the specified resolution and pixel format.
 *
 * This function sets the display to a specific video mode based on the given width, height,
 * and pixel format. It uses the Graphics Output Protocol to find and set the desired mode.
 *
 * Parameters:
 *   protocol - Pointer to the EFI_GRAPHICS_OUTPUT_PROTOCOL instance.
 *   target_width - The desired width of the video mode.
 *   target_height - The desired height of the video mode.
 *   target_pixel_format - The desired pixel format (color format) of the video mode.
 *
 * Returns:
 *   EFI_STATUS - The status of the operation. Returns EFI_SUCCESS if the video mode is successfully set.
 *                Returns specific error codes in case of failures.
 */
EFI_STATUS set_graphics_mode(IN EFI_GRAPHICS_OUTPUT_PROTOCOL *const protocol,
                             IN const UINT32 target_width,
                             IN const UINT32 target_height,
                             IN const EFI_GRAPHICS_PIXEL_FORMAT target_pixel_format)
{
    EFI_STATUS status;
    UINTN graphics_mode_num = 0;

    // Find a video mode that matches the target parameters.
    status = find_video_mode(protocol, target_width, target_height, target_pixel_format, &graphics_mode_num);
    if (EFI_ERROR(status))
    {
        // Error message has already been printed.
        return status;
    }

    // Set the graphics mode to the found video mode.
    status = uefi_call_wrapper(protocol->SetMode, 2, protocol, graphics_mode_num);
    if (EFI_ERROR(status))
    {
        Print(L"Error: Failed to set graphics mode: %r\n", status);
        return status;
    }

    return EFI_SUCCESS;
}

/**
 * init_file_system_service - Initializes the Simple File System Protocol service.
 *
 * This function locates and initializes the Simple File System Protocol, which is
 * necessary for file system operations like reading and writing files in UEFI applications.
 * This is typically used for accessing files on a UEFI-bootable filesystem.
 *
 * Returns:
 *   EFI_STATUS - The status of the operation. Returns EFI_SUCCESS if the Simple File System
 *                Protocol is successfully located. Returns specific error codes in case of failures.
 */
EFI_STATUS init_file_system_service(void)
{
    Print(L"Initialising File System service\n");

    // Locate the Simple File System Protocol.
    EFI_STATUS status = uefi_call_wrapper(gBS->LocateProtocol, 3, &gEfiSimpleFileSystemProtocolGuid, NULL, &fs_protocol);
    if (EFI_ERROR(status))
    {
        Print(L"Fatal Error: Error locating Simple File System Protocol: %r\n", status);
        return status;
    }

    Print(L"Located Simple File System Protocol\n");

    return status;
}

/**
 * load_ps1_font - Loads a PSF1 font file into memory.
 *
 * This function opens a PSF1 font file, validates its header, and loads the font
 * glyphs into a buffer. It then constructs a PSF1_FONT structure containing the
 * header and the glyph buffer.
 *
 * Parameters:
 *   root_file_system - The root file system from which the font is to be loaded.
 *   directory - The directory in which the font file is located.
 *   path - The path to the font file.
 *   image_handle - The handle of the image.
 *   system_table - Pointer to the EFI system table.
 *
 * Returns:
 *   PSF1_FONT* - A pointer to the loaded PSF1_FONT structure, or NULL if loading fails.
 */
PSF1_FONT *load_ps1_font(EFI_FILE *root_file_system, EFI_FILE *directory, CHAR16 *path, EFI_HANDLE image_handle, EFI_SYSTEM_TABLE *system_table)
{
    EFI_STATUS status;
    EFI_FILE *file; // File handle for the font file.

    Print(L"Opening font file: %s\n", path);

    status = uefi_call_wrapper(root_file_system->Open, 5, root_file_system, &file, path, EFI_FILE_MODE_READ, 0);
    if (status != EFI_SUCCESS)
    {
        Print(L"Failed to open file: %s\n", path);
        return NULL;
    }

    PSF1_HEADER *font_header; // Pointer to the font header structure.

    // Allocate memory for the font header.
    status = uefi_call_wrapper(gBS->AllocatePool, 3, EfiLoaderData, sizeof(PSF1_HEADER), (void **)&font_header);
    if (status != EFI_SUCCESS)
    {
        Print(L"Failed to allocate pool: %r\n", status);
        return NULL;
    }

    UINTN size = sizeof(PSF1_HEADER); // Size of the font header.
    // Read the font header from the file.
    status = uefi_call_wrapper(file->Read, 3, file, &size, font_header);
    if (status != EFI_SUCCESS)
    {
        Print(L"Failed to read file: %r\n", status);
        return NULL;
    }

    // Validate the PSF1 magic number.
    if (font_header->magic[0] != 0x36 || font_header->magic[1] != 0x04)
    {
        Print(L"Invalid PSF1 magic\n");
        return NULL;
    }

    // Calculate the size of the glyph buffer based on the font mode.
    UINTN glyph_buffer_size = font_header->char_size * 256;
    if (font_header->mode == 1)
    {
        Print(L"Reading Unicode PSF1\n");
        glyph_buffer_size = font_header->char_size * 512;
    }

    // Allocate memory for the glyph buffer.
    void *glyph_buffer;
    // Set the file position to the start of the glyph buffer.
    status = uefi_call_wrapper(file->SetPosition, 2, file, sizeof(PSF1_HEADER));
    if (status != EFI_SUCCESS)
    {
        Print(L"Failed to set position: %r\n", status);
        return NULL;
    }

    // Allocate memory for the glyph buffer.
    status = uefi_call_wrapper(gBS->AllocatePool, 3, EfiLoaderData, glyph_buffer_size, (void **)&glyph_buffer);
    if (status != EFI_SUCCESS)
    {
        Print(L"Failed to allocate pool: %r\n", status);
        return NULL;
    }

    // Read the glyph buffer from the file.
    status = uefi_call_wrapper(file->Read, 3, file, &glyph_buffer_size, glyph_buffer);
    if (status != EFI_SUCCESS)
    {
        Print(L"Failed to read file: %r\n", status);
        return NULL;
    }

    PSF1_FONT *font; // Pointer to the PSF1 font structure.
    // Allocate memory for the PSF1 font structure.
    status = uefi_call_wrapper(gBS->AllocatePool, 3, EfiLoaderData, sizeof(PSF1_FONT), (void **)&font);
    if (status != EFI_SUCCESS)
    {
        Print(L"Failed to allocate pool: %r\n", status);
        return NULL;
    }

    // Construct the PSF1 font structure.
    font->psf1_header = *font_header;
    font->glyph_buffer = glyph_buffer;

    Print(L"Font loaded successfully\n");

    return font;
}

void *find_rsdp(EFI_SYSTEM_TABLE *SystemTable)
{
    EFI_CONFIGURATION_TABLE *configTable = SystemTable->ConfigurationTable;
    void *rsdp = NULL;

    Print(L"Searching for RSDP\n");

    for (UINTN index = 0; index < SystemTable->NumberOfTableEntries; index++)
    {
        if (CompareGuid(&configTable[index].VendorGuid, &Acpi2TableGuid))
        {
            // Correct the pointer dereference to access VendorTable
            if (compareMemory((CHAR8 *)configTable[index].VendorTable, "RSD PTR ", 8) == 0)
            {
                rsdp = configTable[index].VendorTable;
                break;
            }
        }
    }
    return rsdp;
}

/**
 * efi_main - The entry point for the UEFI application.
 *
 * This function performs the initializations necessary for a UEFI application,
 * such as disabling the watchdog timer, setting up graphics, initializing file
 * system services, and loading the kernel into memory. It then hands over control
 * to the loaded kernel.
 *
 * Parameters:
 *   ImageHandle - The firmware-allocated handle for the EFI image.
 *   SystemTable - A pointer to the EFI system table.
 *
 * Returns:
 *   EFI_STATUS - The status of the operation. Returns EFI_SUCCESS if the operation is successful,
 *                EFI_LOAD_ERROR if the kernel fails to load or execute, or specific error codes in case of other failures.
 */
EFI_STATUS EFIAPI efi_main(EFI_HANDLE ImageHandle,
                           EFI_SYSTEM_TABLE *SystemTable)
{
    EFI_STATUS status;
    EFI_GRAPHICS_OUTPUT_PROTOCOL *graphics_output_protocol = NULL; // Graphics output protocol instance.
    EFI_FILE *root_file_system;                                    // Root file system instance.
    EFI_PHYSICAL_ADDRESS *kernel_entry_point = 0;                  // Kernel entry point address.
    EFI_MEMORY_DESCRIPTOR *memory_map = NULL;                      // Memory map pointer.
    UINTN memory_map_key = 0;                                      // Memory map key.
    UINTN memory_map_size = 0;                                     // Memory map size.
    UINTN descriptor_size;                                         // Size of an individual memory descriptor.
    UINT32 descriptor_version;                                     // Memory descriptor version.
    // void *rsdp = NULL;                                             // Root System Description Pointer.
    void (*kernel_entry)(Boot_Info *boot_info); // Kernel entry function pointer.
    Boot_Info boot_info;                        // Boot information structure.

    fs_protocol = NULL;

    // Initialize the UEFI library.
    InitializeLib(ImageHandle, SystemTable);

    // Disable the watchdog timer.
    status = uefi_call_wrapper(gBS->SetWatchdogTimer, 4, 0, 0, 0, NULL);
    if (EFI_ERROR(status))
    {
        Print(L"Error: Failed to disable watchdog timer: %r\n", status);
        return status;
    }

    // Reset the console input.
    status = uefi_call_wrapper(ST->ConIn->Reset, 2, SystemTable->ConIn, FALSE);
    if (EFI_ERROR(status))
    {
        Print(L"Error: Failed to reset console input: %r\n", status);
        return status;
    }

    // Initialize the graphics output service (GOP)
    status = init_gop();
    if (EFI_ERROR(status))
    {
        Print(L"Error: Failed to initialise graphics output service: %r\n", status);
        return status;
    }

    // Open the graphics output protocol.
    status = uefi_call_wrapper(gBS->OpenProtocol, 6,
                               ST->ConsoleOutHandle, &gEfiGraphicsOutputProtocolGuid,
                               &graphics_output_protocol, ImageHandle,
                               NULL, EFI_OPEN_PROTOCOL_BY_HANDLE_PROTOCOL);
    if (EFI_ERROR(status))
    {
        Print(L"Error: Failed to open graphics output protocol: %r\n", status);
        return status;
    }

    // Set the graphics mode to the target resolution and pixel format.
    status = set_graphics_mode(graphics_output_protocol, TARGET_SCREEN_WIDTH, TARGET_SCREEN_HEIGHT, TARGET_PIXEL_FORMAT);
    if (EFI_ERROR(status))
    {
        return status;
    }

    // Initialize the file system service.
    status = init_file_system_service();
    if (EFI_ERROR(status))
    {
        Print(L"Error: Failed to initialise file system service: %r\n", status);
        return status;
    }

    // Open the root file system volume.
    status = uefi_call_wrapper(fs_protocol->OpenVolume, 2, fs_protocol, &root_file_system);
    if (EFI_ERROR(status))
    {
        Print(L"Error: Failed to open file system volume: %r\n", status);
        return status;
    }

    Print(L"Loading kernel image\n");
    status = load_kernel_image(root_file_system, L"\\kernel.elf", kernel_entry_point);
    if (EFI_ERROR(status))
    {
        // Error message has already been printed.
        return status;
    }

    Print(L"Set Kernel Entry Point to: '0x%llx'\n", *kernel_entry_point);

    void *rsdp = find_rsdp(SystemTable);
    if (rsdp == NULL)
    {
        Print(L"Error: Failed to find RSDP\n");
        return EFI_LOAD_ERROR;
    }

    PSF1_FONT *font = load_ps1_font(root_file_system, NULL, L"\\zap-light16.psf", ImageHandle, SystemTable);
    if (font == NULL)
    {
        Print(L"Failed to load font\n");
        return EFI_LOAD_ERROR;
    }

    Print(L"Kernel end: 0x%llx\n", kernel_end);

    boot_info.framebuffer.pointer =
        (VOID *)graphics_output_protocol->Mode->FrameBufferBase;
    boot_info.framebuffer.width =
        graphics_output_protocol->Mode->Info->HorizontalResolution;
    boot_info.framebuffer.height =
        graphics_output_protocol->Mode->Info->VerticalResolution;
    boot_info.framebuffer.pixels_per_scanline =
        graphics_output_protocol->Mode->Info->PixelsPerScanLine;
    boot_info.font = *font;
    boot_info.rsdp = rsdp;
    boot_info.kernel_end = kernel_end;

    Print(L"Freeing GOP handle buffer\n");
    status = uefi_call_wrapper(gBS->FreePool, 1, graphic_handle_buffer);
    if (EFI_ERROR(status))
    {
        Print(L"Error: Failed to free GOP handle buffer: %r\n", status);
        return status;
    }

    Print(L"Get the memory map prior to exiting the boot service\n");

    // Get the memory map prior to exiting the boot service.
    status = get_memory_map((VOID **)&memory_map, &memory_map_size, &memory_map_key, &descriptor_size, &descriptor_version);
    if (EFI_ERROR(status))
    {
        // Error has already been printed.
        return status;
    }

    status = uefi_call_wrapper(gBS->ExitBootServices, 2, ImageHandle, memory_map_key);
    if (EFI_ERROR(status))
    {
        // Error has already been printed.
        return status;
    }

    boot_info.memory_map = memory_map;
    boot_info.memory_map_size = memory_map_size;
    boot_info.memory_map_descriptor_size = descriptor_size;

    // Set boot information and call the kernel entry function
    kernel_entry = (void (*)(Boot_Info *)) * kernel_entry_point;
    kernel_entry(&boot_info);

    // If kernel_entry returns, it's an error
    return EFI_LOAD_ERROR;
}
