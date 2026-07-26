/* SPDX-License-Identifier: MIT */
/* Userspace/kernel uAPI for Hermes nvidia-drm character device. */
#ifndef HERMES_DRM_UAPI_H
#define HERMES_DRM_UAPI_H

#ifdef HERMES_HOST_TEST
#include <stdint.h>
typedef uint8_t __u8;
typedef uint32_t __u32;
typedef uint64_t __u64;
/* Host tests only exercise logic functions, not ioctl numbers. */
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

/* EDID property blob (128-byte base EDID when Online). */
#define HERMES_DRM_EDID_MAX 128

struct hermes_drm_edid {
	__u32 connector_id;
	__u32 size; /* out: bytes written (0 if offline/empty) */
	__u8 data[HERMES_DRM_EDID_MAX];
};

/* Named property get (EDID blob id / mode count style shell). */
struct hermes_drm_prop_get {
	__u32 object_id; /* connector id for connector props */
	__u32 prop_id; /* 1 = EDID blob id, 2 = DPMS, 3 = CRTC_ID */
	__u64 value; /* out */
};

#define HERMES_DRM_PROP_EDID 1
#define HERMES_DRM_PROP_DPMS 2
#define HERMES_DRM_PROP_CRTC_ID 3

#define HERMES_DRM_IOCTL_STATUS \
	_IOR(HERMES_DRM_IOCTL_BASE, 0x01, struct hermes_drm_status)
#define HERMES_DRM_IOCTL_DUMB_CREATE \
	_IOWR(HERMES_DRM_IOCTL_BASE, 0x02, struct hermes_drm_dumb_create)
#define HERMES_DRM_IOCTL_ATOMIC \
	_IOWR(HERMES_DRM_IOCTL_BASE, 0x03, struct hermes_drm_atomic_req)
#define HERMES_DRM_IOCTL_DISABLE_CRTC \
	_IOWR(HERMES_DRM_IOCTL_BASE, 0x04, __u32)
#define HERMES_DRM_IOCTL_GET_EDID \
	_IOWR(HERMES_DRM_IOCTL_BASE, 0x05, struct hermes_drm_edid)
#define HERMES_DRM_IOCTL_GET_PROP \
	_IOWR(HERMES_DRM_IOCTL_BASE, 0x06, struct hermes_drm_prop_get)

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
	unsigned edid_blob_id;
	unsigned preferred_hdisplay;
	unsigned preferred_vdisplay;
};

void hermes_drm_logic_init(struct hermes_drm_logic *L, int gsp_online);
int hermes_drm_logic_status(const struct hermes_drm_logic *L,
			    struct hermes_drm_status *st);
int hermes_drm_logic_dumb_create(struct hermes_drm_logic *L,
				 struct hermes_drm_dumb_create *req);
int hermes_drm_logic_atomic(struct hermes_drm_logic *L,
			    struct hermes_drm_atomic_req *req);
int hermes_drm_logic_disable(struct hermes_drm_logic *L, __u32 crtc_id);
int hermes_drm_logic_get_edid(struct hermes_drm_logic *L,
			      struct hermes_drm_edid *edid);
int hermes_drm_logic_get_prop(struct hermes_drm_logic *L,
			      struct hermes_drm_prop_get *prop);
/* Pure EDID builder (checksummed 128-byte base). */
void hermes_drm_build_base_edid(__u8 out[HERMES_DRM_EDID_MAX],
				unsigned hdisplay, unsigned vdisplay);

#endif /* HERMES_DRM_UAPI_H */
