# Out-of-tree Hermes GSP modules with NVIDIA-compatible names.
# Build: make -C /lib/modules/$(uname -r)/build M=$(pwd) modules

obj-m += nvidia.o
obj-m += nvidia-modeset.o
obj-m += nvidia-uvm.o
obj-m += nvidia-drm.o
obj-m += nvidia-peermem.o

nvidia-y := hermes_core_mod.o hermes_bringup.o nvidia_main.o
nvidia-modeset-y := hermes_modeset_main.o
nvidia-uvm-y := hermes_uvm_main.o
nvidia-drm-y := hermes_drm_main.o
nvidia-peermem-y := hermes_peermem_main.o

ccflags-y += -I$(src)/include

all:
	$(MAKE) -C /lib/modules/$(shell uname -r)/build M=$(CURDIR) modules

clean:
	$(MAKE) -C /lib/modules/$(shell uname -r)/build M=$(CURDIR) clean

.PHONY: all clean
