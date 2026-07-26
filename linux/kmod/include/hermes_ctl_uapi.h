/* SPDX-License-Identifier: MIT */
/* Userspace/kernel status for /dev/nvidiactl */
#ifndef HERMES_CTL_UAPI_H
#define HERMES_CTL_UAPI_H

#ifdef HERMES_HOST_TEST
#include <stdint.h>
typedef uint32_t __u32;
#else
#include <linux/types.h>
#endif

#define HERMES_CTL_STATUS_VERSION 2

struct hermes_ctl_status {
	__u32 gsp_online;
	__u32 phase;
	__u32 version;
	__u32 module_mask; /* bit0 nvidia,1 modeset,2 uvm,3 drm,4 peermem */
};

#define HERMES_MOD_NVIDIA (1u << 0)
#define HERMES_MOD_MODESET (1u << 1)
#define HERMES_MOD_UVM (1u << 2)
#define HERMES_MOD_DRM (1u << 3)
#define HERMES_MOD_PEERMEM (1u << 4)

static inline void hermes_ctl_status_fill(struct hermes_ctl_status *st, int online,
					  unsigned phase, unsigned module_mask)
{
	st->gsp_online = online ? 1 : 0;
	st->phase = phase;
	st->version = HERMES_CTL_STATUS_VERSION;
	st->module_mask = module_mask;
}

#endif
