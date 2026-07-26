# Out-of-tree Hermes GSP modules with NVIDIA-compatible names.
# Build: make -C /lib/modules/$(uname -r)/build M=$(pwd) modules

obj-m += nvidia.o
obj-m += nvidia-modeset.o
obj-m += nvidia-uvm.o
obj-m += nvidia-drm.o
obj-m += nvidia-peermem.o

nvidia-y := hermes_core_mod.o hermes_bringup.o hermes_chardev.o nvidia_main.o
nvidia-modeset-y := hermes_modeset_main.o
nvidia-uvm-y := hermes_uvm_main.o
nvidia-drm-y := hermes_drm_logic.o hermes_drm_main.o
nvidia-peermem-y := hermes_peermem_main.o

ccflags-y += -I$(src)/include

all:
	$(MAKE) -C /lib/modules/$(shell uname -r)/build M=$(CURDIR) modules

clean:
	$(MAKE) -C /lib/modules/$(shell uname -r)/build M=$(CURDIR) clean

# Host unit tests (no kernel headers required beyond libc).
host-test:
	$(CC) -std=c11 -Wall -Wextra -O2 -DHERMES_HOST_TEST \
		-I. -Iinclude \
		tests/test_bringup_host.c hermes_bringup.c -o tests/test_bringup_host
	$(CC) -std=c11 -Wall -Wextra -O2 -DHERMES_HOST_TEST \
		-I. -Iinclude \
		tests/test_drm_host.c hermes_drm_logic.c -o tests/test_drm_host
	$(CC) -std=c11 -Wall -Wextra -O2 -DHERMES_HOST_TEST \
		-I. -Iinclude \
		tests/test_ctl_status_host.c -o tests/test_ctl_status_host
	$(CC) -std=c11 -Wall -Wextra -O2 \
		tests/test_companion_gate_host.c -o tests/test_companion_gate_host
	./tests/test_bringup_host
	./tests/test_drm_host
	./tests/test_ctl_status_host
	./tests/test_companion_gate_host

.PHONY: all clean host-test
