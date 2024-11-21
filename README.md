# Redox OS test suite

- This branch is a tutorial for running lmbench(rust reimplemented ) under Redox OS with Raspberry Pi 3b+
- Why not Lmbench c code?
  - Redox does not suport `pselect6` which is crucial to run lmbench
- Why do i need this tutorial
  - The official tutorial of running Redox on  Raspberry Pi 3b+ is somehow broken, many things needed to be changed in order to boot Redox. This tutorial saves you the effort  of figuring out how to boot Redox on a Raspberry Pi up to ion(user shell).


## Prerequisite

- Follow the official  Redox book, construct a Redox build system:
  - https://doc.redox-os.org/book/building-redox.html#preparing-the-build
  - after the `time make all` commond, the build system is constructed and we can move on to the next step
- Follow the official tutorial: https://doc.redox-os.org/book/raspi.html#raspberry-pi-3-model-b . 
  - Clone the `redox_firmware` repo
  - Clone the `raspberrypi/firmware` repo
- Now your `tryredox` directory should look like this 
  - `firmware  native_bootstrap.sh  patches  redox  redox_firmware  scripts`


## Changes needed to be done 

- Nearly all changes lie under `patches/`. You can simple apply thouse patches to the corresponding git repo.
- The changes are listed as follows.

### dts

-  `bcm2837-rpi-3-b-plus.dts` inside `redox_firmware` repo
- Patch file:`dts.patch`

1. **UART Node**:
   - Modified the `interrupts` property by adding a new interrupt value `<0x25>`.

2. **Interrupt Controller**:
   - Modified the `interrupts` property by adding an additional interrupt value `<0x0>`.

3. **SoC Node**:
   - Added new properties `#address-cells` and `#size-cells` with values `<0x01>` each to the `soc` node. 

### build system configuration 

- add recipe `lmbench` in minimal.toml

- mount sdcard in `qemu.mk` 

  ```
  qemu_raspi: qemu-deps
  	$(QEMU) -M raspi3b -smp 4,cores=1 \
  		-kernel $(FIRMWARE) \
  		-serial stdio -display none \
  		-sd $(DISK)
  ```

- add configurations in `.config`

  ```
  PODMAN_BUILD?=0
  ARCH?=aarch64
  CONFIG_NAME?=minimal
  CONFIG?=minimal
  BOARD?=raspi3bp
  ```

### Kernel

- Path:`tryredox/redox/cookbook/recipes/core/kernel/source`
- Redox's `sys_clockgettime` for aarch64 is an empty implementation which only returns 0. We need to get this done.

### bcm2835 driver

- path:`tryredox/redox/cookbook/recipes/core/drivers-initfs/source/storage/bcm2835-sdhcid`

- The emmc driver is missing in the `driver-initfs`. Add it in the `drivers-initfs/recipe.toml->line64`

  ```
  - pcid | fbcond | inputd | vesad | lived | ps2d | acpid )
  + pcid | fbcond | inputd | vesad | lived | ps2d | acpid | bcm2835-sdhcid)
  ```

- Redox's driver for bcm2835 emmc doesn't work for me. SInce Uboot has already initialized emmc once, we don't need to do that again.

  - __If you intend to test on qemu, do not modify source code of bcm2835-sdhcid__


### bootloader

- Path:`tryredox/redox/cookbook/recipes/core/bootloader`
- This one is weird . We need to add a println before `area_add(entry);` ,or Redox won't be able to find any free memory

## Lmbench recipe

- In order to run a Rust program in Redox,we need to add it under the recipe folder
- run command `cp -r lmbench tryredox/redox/cookbook/recipes`
- Then, run `make rebuild` to rebuild the kernel with changes made and a new recipe added.

## Run on qemu or a Raspiberry pi

- You can either follow the tutorial https://doc.redox-os.org/book/raspi.html#raspberry-pi-3-model-b  or use the scripts under `scripts/`
- Copy the scripts to `tryredox/redox/` and change the `WORKPLACE` environment variable
- Alwasys log in as `root`

### qemu

- run `sudo bash qemu.sh`
- run `make qemu_raspi live=no` 

### Raspiberry pi 3b+

- I use a `SanDisk Ultra microSDHC 32GB` sdcard
- change `SDPATH` to your ow path(/dev/sdx)
- run `sudo bash rasp.sh`
- insert the card in the board and hit the power button, make sure you have properly connected the serial output.



## Usage

- after login, cd to `/usr/bin`. You should find `lmbench` under this path.
- `./lmbench` + tests you want to run. 

```Rust
//Redox supported lmbench. These tests are reimplemented in rust 
./lmbench null
./lmbench ctx
./lmbench bw_file_rd
./lmbench lat_fs
./lmbench lat_pipe
//./lmbench lat_mmap nKBs mmap_size,default is 4KB 
./lmbench lat_mmap 512
//same as lat_mmap
./lmbench bw_mmap_rd 512 mmap_only
./lmbench bw_mmap_rd 512 open2close
//first arg is packet_size, nKBs,
//second arg is total_bytes,nMBs.
./lmbench bw_pipe 1024 1024
```

- wait for results.

  - an example of null syscall test

    ```
    0;root: /usr/binroot:/usr/bin# ./lmbench null
    get_timer_value() overhead is 9506 microseconds (0.9506 microsecods / iteration)
    get_timer_value() overhead is 9506 microseconds (0.9506 microsecods / iteration)
    get_timer_value() overhead is 9506 microseconds (0.9506 microsecods / iteration)
    get_timer_value() overhead is 9506 microseconds (0.9506 microsecods / iteration)
    get_timer_value() overhead is 9506 microseconds (0.9506 microsecods / iteration)
    get_timer_value() overhead is 9506 microseconds (0.9506 microsecods / iteration)
    get_timer_value() overhead is 9506 microseconds (0.9506 microsecods / iteration)
    get_timer_value() overhead is 9506 microseconds (0.9506 microsecods / iteration)
    get_timer_value() overhead is 9506 microseconds (0.9506 microsecods / iteration)
    get_timer_value() overhead is 9506 microseconds (0.9506 microsecods / iteration)
    get_timer_value() overhead is 0.951 microseconds, original tries 9.506
    ========================================
    Time unit : micro_sec
    Iterations: 100000
    Tries     : 10
    ========================================
    null_test_inner (1/10): overhead 0.951, 66334.049 total_time -> 0.663
    null_test_inner (2/10): overhead 0.951, 66334.049 total_time -> 0.663
    null_test_inner (3/10): overhead 0.951, 66334.049 total_time -> 0.663
    null_test_inner (4/10): overhead 0.951, 66334.049 total_time -> 0.663
    null_test_inner (5/10): overhead 0.951, 66334.049 total_time -> 0.663
    null_test_inner (6/10): overhead 0.951, 66334.049 total_time -> 0.663
    null_test_inner (7/10): overhead 0.951, 66333.049 total_time -> 0.663
    null_test_inner (8/10): overhead 0.951, 66334.049 total_time -> 0.663
    null_test_inner (9/10): overhead 0.951, 66334.049 total_time -> 0.663
    null_test_inner (10/10): overhead 0.951, 66334.049 total_time -> 0.663
    Null syscall test completed successfully.
    Stats
    
            min:     0.663
    
            p_25:    0.663
    
            median:  0.663
    
            p_75:    0.663
    
            max:     0.663
    
            mode:    0.660
    
            mean:    0.663
    
            std_dev: 0.000
    
    This test is equivalent to `lat_syscall null` in LMBench
    ```

    

    
