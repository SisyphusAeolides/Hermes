/* SPDX-License-Identifier: MIT */
/* Companion chardev uAPI — modeset / uvm / peermem. */
#ifndef HERMES_COMPANION_UAPI_H
#define HERMES_COMPANION_UAPI_H

#include "hermes_ctl_uapi.h"

#ifndef HERMES_HOST_TEST
#define HERMES_COMPANION_IOCTL_STATUS \
	_IOR(HERMES_CTL_IOCTL_BASE, 0x20, struct hermes_ctl_status)
#define HERMES_UVM_IOCTL_STATUS HERMES_COMPANION_IOCTL_STATUS

/* UVM software surface (Online-gated). */
struct hermes_uvm_init {
	__u32 flags;
	__u32 reserved;
};

struct hermes_uvm_register_gpu {
	__u32 gpu_uuid[4]; /* 16 bytes */
	__u32 rm_ctrl_fd; /* unused shell */
	__u32 registered; /* out */
};

#define HERMES_UVM_IOCTL_INITIALIZE \
	_IOW(HERMES_CTL_IOCTL_BASE, 0x21, struct hermes_uvm_init)
#define HERMES_UVM_IOCTL_PAGEABLE_MEM_ACCESS \
	_IOR(HERMES_CTL_IOCTL_BASE, 0x22, __u32)
#define HERMES_UVM_IOCTL_REGISTER_GPU \
	_IOWR(HERMES_CTL_IOCTL_BASE, 0x23, struct hermes_uvm_register_gpu)
#define HERMES_UVM_IOCTL_UNREGISTER_GPU \
	_IOW(HERMES_CTL_IOCTL_BASE, 0x24, __u32)

/* Modeset software surface (Online-gated). */
struct hermes_modeset_alloc {
	__u32 width;
	__u32 height;
	__u32 handle; /* out */
};

struct hermes_modeset_flip {
	__u32 handle;
	__u32 crtc_id;
	__u32 sequence; /* out */
};

#define HERMES_MODESET_IOCTL_ALLOC \
	_IOWR(HERMES_CTL_IOCTL_BASE, 0x30, struct hermes_modeset_alloc)
#define HERMES_MODESET_IOCTL_FLIP \
	_IOWR(HERMES_CTL_IOCTL_BASE, 0x31, struct hermes_modeset_flip)
#define HERMES_MODESET_IOCTL_FREE \
	_IOW(HERMES_CTL_IOCTL_BASE, 0x32, __u32)
#endif

#endif
