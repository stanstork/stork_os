(cd bootloader ; nasm -o boot boot.asm)
boot_result=$?

(make clean)
(make)
make_result=$?

echo Make Result: $make_result

if [ "$boot_result" = "0" ] && [ "$make_result" = "0" ]
then
    mv bootloader/boot build/os.img
    cat build/kernel.bin >> build/os.img

    fsize=$(wc -c < build/os.img)
    sectors=$(( $fsize / 512 ))

    echo "Build finished successfully"
    echo "ALERT: Adjust boot sector to load $sectors sectors"
else
    result=`expr $boot_result + $make_result`
    echo "Build failed with error code $result. See output for more info."
fi