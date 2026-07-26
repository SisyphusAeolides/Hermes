/* SPDX-License-Identifier: MIT */
/* Userspace/kernel status for /dev/nvidiactl */
#ifndef HERMES_CTL_UAPI_H
#define HERMES_CTL_UAPI_H

#ifdef HERMES_HOST_TEST
#include <stdint.h>
typedef uint32_t __u32;
#else
#include <linux/types.h>
#include <linux/ioctl.h>
#endif

#define HERMES_CTL_STATUS_VERSION 3

struct hermes_ctl_status {
	__u32 gsp_online;
	__u32 phase;
	__u32 version;
	__u32 module_mask; /* bit0 nvidia,1 modeset,2 uvm,3 drm,4 peermem */
};

#define HERMES_CTL_IOCTL_BASE 0x48 /* 'H' */

#ifndef HERMES_HOST_TEST
#define HERMES_CTL_IOCTL_STATUS \
	_IOR(HERMES_CTL_IOCTL_BASE, 0x10, struct hermes_ctl_status)
/*
 * SIM_PROMOTE: complete-evidence bring-up on first Turing+ NVIDIA PCI GPU.
 * Requires module_param allow_sim_promote=1. Integration gate only — not a
 * claim of measured silicon. DEMOTE forces Offline again.
 */
#define HERMES_CTL_IOCTL_SIM_PROMOTE _IO(HERMES_CTL_IOCTL_BASE, 0x11)
#define HERMES_CTL_IOCTL_DEMOTE _IO(HERMES_CTL_IOCTL_BASE, 0x12)
#endif

#define HERMES_MOD_NVIDIA (1u << 0)
#define HERMES_MOD_MODESET (1u << 1)
#define HERMES_MOD_UVM (1u << 2)
#define HERMES_MOD_DRM (1u << 3)
#define HERMES_MOD_PEERMEM (1u << 4)

#define HERMES_MOD_ALL_OPEN_STACK                                              \
	(HERMES_MOD_NVIDIA | HERMES_MOD_MODESET | HERMES_MOD_UVM |             \
	 HERMES_MOD_DRM | HERMES_MOD_PEERMEM)

/*
 * Compose a mask from live companion presence flags (1 = present).
 * Primary nvidia is always bit0 when this code runs inside nvidia.ko.
 * Host-testable pure combinator (no kernel).
 */
static inline unsigned hermes_ctl_module_mask_compose(int modeset, int uvm,
						     int drm, int peermem)
{
	unsigned m = HERMES_MOD_NVIDIA;

	if (modeset)
		m |= HERMES_MOD_MODESET;
	if (uvm)
		m |= HERMES_MOD_UVM;
	if (drm)
		m |= HERMES_MOD_DRM;
	if (peermem)
		m |= HERMES_MOD_PEERMEM;
	return m;
}

static inline void hermes_ctl_status_fill(struct hermes_ctl_status *st, int online,
					  unsigned phase, unsigned module_mask)
{
	st->gsp_online = online ? 1 : 0;
	st->phase = phase;
	st->version = HERMES_CTL_STATUS_VERSION;
	st->module_mask = module_mask;
}

#endif
