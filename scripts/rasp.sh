DISK=build/aarch64/minimal/harddrive.img
MOUNT_DIR=/mnt/efi_boot
DTB_DIR=$MOUNT_DIR/dtb/broadcom
WORKPLACE=/home/zhangxb/try-redox
DTS=$WORKPLACE/redox_firmware/platform/raspberry_pi/rpi3/bcm2837-rpi-3-b-plus.dts
UBOOT=$WORKPLACE/redox_firmware/platform/raspberry_pi/rpi3/u-boot-rpi-3-b-plus.bin
CONFIG_TXT=$WORKPLACE/redox_firmware/platform/raspberry_pi/rpi3/config.txt
FW_DIR=$WORKPLACE/firmware/boot
SDPATH=/dev/sda

dtc -I dts -O dtb $DTS > ./bcm2837-rpi-3-b.dtb

sudo mkdir -p $MOUNT_DIR

sudo mount -o loop,offset=$((2048*512)) $DISK $MOUNT_DIR

cp -rf $FW_DIR/* $MOUNT_DIR

mkdir -p $DTB_DIR

sudo cp  bcm2837-rpi-3-b.dtb $DTB_DIR/bcm2837-rpi-3-b.dtb

sudo cp $DTB_DIR/bcm2837-rpi-3-b.dtb $DTB_DIR/bcm2837-rpi-3-b-plus.dtb

cp $UBOOT $MOUNT_DIR/u-boot.bin

cp $CONFIG_TXT $MOUNT_DIR

sync

sudo umount $MOUNT_DIR

dd if=build/aarch64/minimal/harddrive.img of=$SDPATH

sudo gdisk $SDPATH <<EOF
r
p
h
2
n
0c
n
n
o
w
y
y
EOF
