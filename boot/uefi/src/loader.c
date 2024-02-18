#include <efi.h>
#include <efilib.h>
#include <elf.h>

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
	VOID *program_data = NULL;										   // Buffer to read segment data into
	UINTN buffer_read_size = 0;										   // Size of segment data read
	UINTN segment_page_count = EFI_SIZE_TO_PAGES(segment_memory_size); // Number of pages to allocate for segment
	EFI_PHYSICAL_ADDRESS zero_fill_start = 0;						   // Start address of zero fill
	UINTN zero_fill_count = 0;										   // Number of bytes to zero fill

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
	UINTN p = 0;				  // Program header index

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

	program_headers_offset = ((Elf64_Ehdr *)*kernel_header_buffer)->e_phoff;				// Get program headers offset
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
	EFI_FILE *kernel_img_file;			 // Pointer to the kernel image file
	VOID *kernel_header = NULL;			 // Buffer to hold the kernel header
	VOID *kernel_program_headers = NULL; // Buffer to hold the kernel program headers
	UINT8 *elf_identity_buffer = NULL;	 // Buffer to hold the ELF identity

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
