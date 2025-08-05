DISK=build/aarch64/minimal/harddrive.img

MOUNT_DIR=/mnt/efi_boot

DTB_DIR=$MOUNT_DIR/dtb/broadcom

WORKPLACE=/home/ubuntu/LMBench-Rust/tryredox

DTS=$WORKPLACE/redox_firmware/platform/raspberry_pi/rpi3/bcm2837-rpi-3-b-plus.dts

sudo dtc -I dts -O dtb $DTS > ./bcm2837-rpi-3-b.dtb

sudo mkdir -p $MOUNT_DIR

sudo mount -o loop,offset=$((2048*512)) $DISK $MOUNT_DIR

sudo mkdir -p $DTB_DIR

sudo cp bcm2837-rpi-3-b.dtb $DTB_DIR/bcm2837-rpi-3-b.dtb

sudo cp $DTB_DIR/bcm2837-rpi-3-b.dtb $DTB_DIR/bcm2837-rpi-3-b-plus.dtb

sync

sudo umount $MOUNT_DIR

