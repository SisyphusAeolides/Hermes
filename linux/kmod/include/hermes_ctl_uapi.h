/* SPDX-License-Identifier: MIT */
/* Userspace/kernel status for /dev/nvidiactl */
#ifndef HERMES_CTL_UAPI_H
#define HERMES_CTL_UAPI_H

#ifdef HERMES_HOST_TEST
#include <stdint.h>
typedef uint8_t __u8;
typedef uint32_t __u32;
#else
#include <linux/types.h>
#include <linux/ioctl.h>
#endif

#define HERMES_CTL_STATUS_VERSION 4

struct hermes_ctl_status {
	__u32 gsp_online;
	__u32 phase;
	__u32 version;
	__u32 module_mask; /* bit0 nvidia,1 modeset,2 uvm,3 drm,4 peermem */
};

/*
 * MEASURE_FW: userspace supplies host-measured digest + length; kernel pins
 * against embedded OpenRM allow-list (blobs not in kernel). Sets firmware_measured
 * and re-runs bring-up with current evidence (may stop at FIRMWARED if other
 * gates incomplete). Real silicon measure, not invented Online.
 */
struct hermes_measure_fw {
	__u32 byte_length;
	__u8 sha256[32];
	__u32 admitted; /* out: 1 if pin matched */
	__u32 phase; /* out: hermes_phase after apply */
	__u32 online; /* out */
	__u32 status; /* out: hermes_bringup_status */
};

/*
 * APPLY_EVIDENCE: progressive host evidence. firmware_measured only honored if
 * a prior MEASURE_FW admitted (or force_sim with allow_sim_promote).
 * Other bits are operator-asserted host facts; Online still requires all true
 * via hermes_run_bringup (fail-closed).
 */
struct hermes_apply_evidence {
	__u32 iommu_isolated;
	__u32 dma_domain;
	__u32 wpr_locked;
	__u32 mailbox_ok;
	__u32 ready_ok;
	__u32 use_measured_fw; /* 1 = require prior MEASURE_FW admit */
	__u32 force_fw_measured; /* 1 = only if allow_sim_promote (sim) */
	__u32 phase; /* out */
	__u32 online; /* out */
	__u32 status; /* out */
};

#define HERMES_CTL_IOCTL_BASE 0x48 /* 'H' */

#ifndef HERMES_HOST_TEST
#define HERMES_CTL_IOCTL_STATUS \
	_IOR(HERMES_CTL_IOCTL_BASE, 0x10, struct hermes_ctl_status)
#define HERMES_CTL_IOCTL_SIM_PROMOTE _IO(HERMES_CTL_IOCTL_BASE, 0x11)
#define HERMES_CTL_IOCTL_DEMOTE _IO(HERMES_CTL_IOCTL_BASE, 0x12)
#define HERMES_CTL_IOCTL_MEASURE_FW \
	_IOWR(HERMES_CTL_IOCTL_BASE, 0x13, struct hermes_measure_fw)
#define HERMES_CTL_IOCTL_APPLY_EVIDENCE \
	_IOWR(HERMES_CTL_IOCTL_BASE, 0x14, struct hermes_apply_evidence)
#endif

#define HERMES_MOD_NVIDIA (1u << 0)
#define HERMES_MOD_MODESET (1u << 1)
#define HERMES_MOD_UVM (1u << 2)
#define HERMES_MOD_DRM (1u << 3)
#define HERMES_MOD_PEERMEM (1u << 4)

#define HERMES_MOD_ALL_OPEN_STACK                                              \
	(HERMES_MOD_NVIDIA | HERMES_MOD_MODESET | HERMES_MOD_UVM |             \
	 HERMES_MOD_DRM | HERMES_MOD_PEERMEM)

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

/* Host-testable: pin match against allow-list table. */
struct hermes_fw_pin {
	__u32 byte_length;
	__u8 sha256[32];
};

static inline int hermes_fw_pin_match(const struct hermes_fw_pin *pins, unsigned n,
				     __u32 len, const __u8 sha[32])
{
	unsigned i, j;

	if (!pins || !sha)
		return 0;
	for (i = 0; i < n; i++) {
		if (pins[i].byte_length != len)
			continue;
		for (j = 0; j < 32; j++) {
			if (pins[i].sha256[j] != sha[j])
				break;
		}
		if (j == 32)
			return 1;
	}
	return 0;
}

#endif
