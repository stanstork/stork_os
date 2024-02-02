(make clean)
(make)
make_result=$?

echo Make Result: $make_result

if [ "$make_result" = "0" ]
then
    fsize=$(wc -c < build/os-image.bin)
    sectors=$(( $fsize / 512 ))

    echo "Build finished successfully"
    echo "ALERT: Adjust boot sector to load $sectors sectors"
else
    result=`expr $boot_result + $make_result`
    echo "Build failed with error code $result. See output for more info."
    (make clean)
fi