// SPDX-License-Identifier: MIT
/*
 * Host-testable DRM logic for Hermes nvidia-drm.
 * Mirrors hermes-drm fail-closed policy without inventing Online.
 */

#ifdef HERMES_HOST_TEST
#include <stdbool.h>
#include <stdint.h>
#include <string.h>
#define __u32 uint32_t
#define __u64 uint64_t
#else
#include <linux/kernel.h>
#include <linux/string.h>
#endif

#include "include/hermes_drm_uapi.h"

void hermes_drm_logic_init(struct hermes_drm_logic *L, int gsp_online)
{
	if (!L)
		return;
	memset(L, 0, sizeof(*L));
	L->gsp_online = gsp_online ? 1 : 0;
	L->connectors = 1;
	L->crtcs = 1;
	L->active_crtcs = 0;
	L->next_handle = 1;
	L->sequence = 0;
	L->last_fb = 0;
}

int hermes_drm_logic_status(const struct hermes_drm_logic *L,
			    struct hermes_drm_status *st)
{
	if (!L || !st)
		return HERMES_DRM_E_INVAL;
	st->gsp_online = L->gsp_online ? 1 : 0;
	st->connectors = L->connectors;
	st->crtcs = L->crtcs;
	st->active_crtcs = L->active_crtcs;
	st->version = 1;
	return HERMES_DRM_OK;
}

int hermes_drm_logic_dumb_create(struct hermes_drm_logic *L,
				 struct hermes_drm_dumb_create *req)
{
	__u32 pitch;
	__u64 size;

	if (!L || !req)
		return HERMES_DRM_E_INVAL;
	if (!L->gsp_online)
		return HERMES_DRM_E_GSP_OFFLINE;
	if (req->width == 0 || req->height == 0 || req->bpp == 0)
		return HERMES_DRM_E_INVAL;

	pitch = req->width * ((req->bpp + 7) / 8);
	pitch = (pitch + 63u) & ~63u;
	size = (__u64)pitch * (__u64)req->height;
	if (size > (512ull * 1024ull * 1024ull))
		return HERMES_DRM_E_INVAL;

	req->handle = L->next_handle++;
	if (L->next_handle == 0)
		L->next_handle = 1;
	req->pitch = pitch;
	req->size = size;
	return HERMES_DRM_OK;
}

int hermes_drm_logic_atomic(struct hermes_drm_logic *L,
			    struct hermes_drm_atomic_req *req)
{
	if (!L || !req)
		return HERMES_DRM_E_INVAL;
	if (!L->gsp_online)
		return HERMES_DRM_E_GSP_OFFLINE;
	if (req->connector_id == 0 || req->crtc_id == 0 || req->plane_id == 0)
		return HERMES_DRM_E_INVAL;
	if (req->hdisplay == 0 || req->vdisplay == 0)
		return HERMES_DRM_E_INVAL;
	if (req->fb_id == 0 && req->active)
		return HERMES_DRM_E_INVAL;

	if (req->active) {
		if (L->active_crtcs == 0)
			L->active_crtcs = 1;
		L->last_fb = req->fb_id;
	} else {
		L->active_crtcs = 0;
		L->last_fb = 0;
	}
	L->sequence++;
	req->sequence = L->sequence;
	return HERMES_DRM_OK;
}

int hermes_drm_logic_disable(struct hermes_drm_logic *L, __u32 crtc_id)
{
	if (!L)
		return HERMES_DRM_E_INVAL;
	if (!L->gsp_online)
		return HERMES_DRM_E_GSP_OFFLINE;
	if (crtc_id == 0)
		return HERMES_DRM_E_INVAL;
	L->active_crtcs = 0;
	L->last_fb = 0;
	L->sequence++;
	return HERMES_DRM_OK;
}
