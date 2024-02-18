#include <efi.h>
#include <efilib.h>

// Define constants for the target screen width, height, and pixel format.
#define TARGET_SCREEN_WIDTH 1024
#define TARGET_SCREEN_HEIGHT 768
#define TARGET_PIXEL_FORMAT PixelBlueGreenRedReserved8BitPerColor

// Framebuffer structure representing a basic framebuffer.
typedef struct
{
    VOID *pointer;             // Pointer to the beginning of the framebuffer in memory.
    UINT32 width;              // Width of the framebuffer in pixels.
    UINT32 height;             // Height of the framebuffer in pixels.
    UINT32 pixels_per_scaline; // Number of pixels per scanline (often equals width but can be larger).
} Framebuffer;

// Boot_Info structure containing information passed to the OS at boot time.
typedef struct
{
    EFI_MEMORY_DESCRIPTOR *memory_map; // Pointer to the system's memory map.
    UINTN memory_map_size;             // Total size of the memory map.
    UINTN memory_map_descriptor_size;  // Size of an individual memory descriptor in the memory map.
    Framebuffer framebuffer;           // Framebuffer information for the display.
} Boot_Info;

// Global variable to hold the handle buffer for graphics protocols.
EFI_HANDLE *graphic_handle_buffer;

// Global variable to hold the file system protocol interface.
EFI_SIMPLE_FILE_SYSTEM_PROTOCOL *fs_protocol;

// Load a kernel image from the file system and return the entry point.
EFI_STATUS load_kernel_image(IN EFI_FILE *const root_file_system,
                             IN CHAR16 *const kernel_image_filename,
                             OUT EFI_PHYSICAL_ADDRESS *kernel_entry_point);

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

        // Check if the current mode matches the target parameters.
        if (mode_info->HorizontalResolution == target_width &&
            mode_info->VerticalResolution == target_height &&
            mode_info->PixelFormat == target_pixel_format)
        {
            // If a match is found, store the index of the mode and return success.
            *video_mode = i;
            return EFI_SUCCESS;
        }
    }

    // If no matching mode is found, return an error.
    return EFI_UNSUPPORTED;
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
    void (*kernel_entry)(Boot_Info *boot_info);                    // Kernel entry function pointer.
    Boot_Info boot_info;                                           // Boot information structure.

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

    boot_info.framebuffer.pointer =
        (VOID *)graphics_output_protocol->Mode->FrameBufferBase;
    boot_info.framebuffer.width =
        graphics_output_protocol->Mode->Info->HorizontalResolution;
    boot_info.framebuffer.height =
        graphics_output_protocol->Mode->Info->VerticalResolution;
    boot_info.framebuffer.pixels_per_scaline =
        graphics_output_protocol->Mode->Info->PixelsPerScanLine;

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
