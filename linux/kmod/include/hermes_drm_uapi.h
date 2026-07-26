/* SPDX-License-Identifier: MIT */
/* Userspace/kernel uAPI for Hermes nvidia-drm character device. */
#ifndef HERMES_DRM_UAPI_H
#define HERMES_DRM_UAPI_H

#ifdef HERMES_HOST_TEST
#include <stdint.h>
typedef uint32_t __u32;
typedef uint64_t __u64;
#else
#include <linux/types.h>
#include <linux/ioctl.h>
#endif

#define HERMES_DRM_IOCTL_BASE 0x48 /* 'H' */

struct hermes_drm_status {
	__u32 gsp_online;
	__u32 connectors;
	__u32 crtcs;
	__u32 active_crtcs;
	__u32 version;
};

struct hermes_drm_dumb_create {
	__u32 width;
	__u32 height;
	__u32 bpp;
	__u32 handle; /* out */
	__u32 pitch;  /* out */
	__u32 pad;
	__u64 size; /* out */
};

struct hermes_drm_atomic_req {
	__u32 connector_id;
	__u32 crtc_id;
	__u32 plane_id;
	__u32 fb_id;
	__u32 hdisplay;
	__u32 vdisplay;
	__u32 active;
	__u32 sequence; /* out */
};

#define HERMES_DRM_IOCTL_STATUS \
	_IOR(HERMES_DRM_IOCTL_BASE, 0x01, struct hermes_drm_status)
#define HERMES_DRM_IOCTL_DUMB_CREATE \
	_IOWR(HERMES_DRM_IOCTL_BASE, 0x02, struct hermes_drm_dumb_create)
#define HERMES_DRM_IOCTL_ATOMIC \
	_IOWR(HERMES_DRM_IOCTL_BASE, 0x03, struct hermes_drm_atomic_req)
#define HERMES_DRM_IOCTL_DISABLE_CRTC \
	_IOWR(HERMES_DRM_IOCTL_BASE, 0x04, __u32)

/* Host-testable pure logic (also linked into the module). */
enum hermes_drm_logic_err {
	HERMES_DRM_OK = 0,
	HERMES_DRM_E_GSP_OFFLINE = 1,
	HERMES_DRM_E_INVAL = 2,
	HERMES_DRM_E_NOT_ACTIVE = 3,
};

struct hermes_drm_logic {
	int gsp_online;
	unsigned connectors;
	unsigned crtcs;
	unsigned active_crtcs;
	unsigned next_handle;
	unsigned sequence;
	unsigned last_fb;
};

void hermes_drm_logic_init(struct hermes_drm_logic *L, int gsp_online);
int hermes_drm_logic_status(const struct hermes_drm_logic *L,
			    struct hermes_drm_status *st);
int hermes_drm_logic_dumb_create(struct hermes_drm_logic *L,
				 struct hermes_drm_dumb_create *req);
int hermes_drm_logic_atomic(struct hermes_drm_logic *L,
			    struct hermes_drm_atomic_req *req);
int hermes_drm_logic_disable(struct hermes_drm_logic *L, __u32 crtc_id);

#endif /* HERMES_DRM_UAPI_H */
