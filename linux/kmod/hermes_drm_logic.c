// SPDX-License-Identifier: MIT
/*
 * Host-testable DRM logic for Hermes nvidia-drm.
 * Mirrors hermes-drm evidence-gated policy without inventing Online.
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
	L->edid_blob_id = gsp_online ? 1 : 0;
	L->preferred_hdisplay = 1920;
	L->preferred_vdisplay = 1080;
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

void hermes_drm_build_base_edid(__u8 out[HERMES_DRM_EDID_MAX],
				unsigned hdisplay, unsigned vdisplay)
{
	unsigned i;
	unsigned sum = 0;
	unsigned ha, va, hb, vb, clk;
	__u8 *d;

	if (!out)
		return;
	memset(out, 0, HERMES_DRM_EDID_MAX);
	/* Header */
	out[0] = 0x00;
	out[1] = 0xff;
	out[2] = 0xff;
	out[3] = 0xff;
	out[4] = 0xff;
	out[5] = 0xff;
	out[6] = 0xff;
	out[7] = 0x00;
	/* Manufacturer "HRS" compressed */
	out[8] = 0x22;
	out[9] = 0x53;
	out[10] = 0x01;
	out[11] = 0x00;
	out[16] = 1;
	out[17] = 34;
	out[18] = 1;
	out[19] = 4;
	out[20] = 0x80;
	out[21] = 60;
	out[22] = 34;
	out[23] = 120;
	out[24] = 0x0a;

	/* Detailed timing descriptor @54 — simplified FHD-style */
	d = &out[54];
	if (hdisplay == 0)
		hdisplay = 1920;
	if (vdisplay == 0)
		vdisplay = 1080;
	clk = 14850; /* 148.50 MHz / 10 kHz units */
	d[0] = (unsigned char)(clk & 0xff);
	d[1] = (unsigned char)((clk >> 8) & 0xff);
	ha = hdisplay;
	hb = 280; /* blanking shell */
	va = vdisplay;
	vb = 45;
	d[2] = (unsigned char)(ha & 0xff);
	d[3] = (unsigned char)(hb & 0xff);
	d[4] = (unsigned char)(((ha >> 8) & 0xf) << 4 | ((hb >> 8) & 0xf));
	d[5] = (unsigned char)(va & 0xff);
	d[6] = (unsigned char)(vb & 0xff);
	d[7] = (unsigned char)(((va >> 8) & 0xf) << 4 | ((vb >> 8) & 0xf));
	d[17] = 0x1e;

	/* Monitor name descriptor @72 */
	out[72 + 3] = 0xfc;
	out[72 + 5] = 'H';
	out[72 + 6] = 'e';
	out[72 + 7] = 'r';
	out[72 + 8] = 'm';
	out[72 + 9] = 'e';
	out[72 + 10] = 's';
	out[72 + 11] = 0x0a;

	for (i = 0; i < 127; i++)
		sum += out[i];
	out[127] = (unsigned char)((256 - (sum % 256)) & 0xff);
}

int hermes_drm_logic_get_edid(struct hermes_drm_logic *L,
			      struct hermes_drm_edid *edid)
{
	if (!L || !edid)
		return HERMES_DRM_E_INVAL;
	if (edid->connector_id == 0 || edid->connector_id > L->connectors)
		return HERMES_DRM_E_INVAL;
	if (!L->gsp_online) {
		edid->size = 0;
		memset(edid->data, 0, sizeof(edid->data));
		return HERMES_DRM_E_GSP_OFFLINE;
	}
	hermes_drm_build_base_edid(edid->data, L->preferred_hdisplay,
				   L->preferred_vdisplay);
	edid->size = HERMES_DRM_EDID_MAX;
	return HERMES_DRM_OK;
}

int hermes_drm_logic_get_prop(struct hermes_drm_logic *L,
			      struct hermes_drm_prop_get *prop)
{
	if (!L || !prop)
		return HERMES_DRM_E_INVAL;
	if (prop->object_id == 0 || prop->object_id > L->connectors)
		return HERMES_DRM_E_INVAL;
	if (!L->gsp_online)
		return HERMES_DRM_E_GSP_OFFLINE;

	switch (prop->prop_id) {
	case HERMES_DRM_PROP_EDID:
		prop->value = L->edid_blob_id;
		break;
	case HERMES_DRM_PROP_DPMS:
		/* 0 = ON when any CRTC active */
		prop->value = L->active_crtcs ? 0 : 3; /* 3 = OFF */
		break;
	case HERMES_DRM_PROP_CRTC_ID:
		prop->value = L->active_crtcs ? 1 : 0;
		break;
	default:
		return HERMES_DRM_E_INVAL;
	}
	return HERMES_DRM_OK;
}
