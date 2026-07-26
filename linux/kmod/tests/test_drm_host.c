/* Host unit test for hermes_drm_logic (no kernel). */
#include <assert.h>
#include <stdio.h>
#include <string.h>

#define HERMES_HOST_TEST 1
#include "include/hermes_drm_uapi.h"

int main(void)
{
	struct hermes_drm_logic L;
	struct hermes_drm_status st;
	struct hermes_drm_dumb_create dumb;
	struct hermes_drm_atomic_req atom;

	hermes_drm_logic_init(&L, 0);
	assert(hermes_drm_logic_status(&L, &st) == HERMES_DRM_OK);
	assert(st.gsp_online == 0);

	memset(&dumb, 0, sizeof(dumb));
	dumb.width = 1920;
	dumb.height = 1080;
	dumb.bpp = 32;
	assert(hermes_drm_logic_dumb_create(&L, &dumb) == HERMES_DRM_E_GSP_OFFLINE);

	hermes_drm_logic_init(&L, 1);
	assert(hermes_drm_logic_dumb_create(&L, &dumb) == HERMES_DRM_OK);
	assert(dumb.handle == 1);
	assert(dumb.pitch % 64 == 0);
	assert(dumb.size >= 1920ull * 1080ull * 4ull);

	memset(&atom, 0, sizeof(atom));
	atom.connector_id = 1;
	atom.crtc_id = 1;
	atom.plane_id = 1;
	atom.fb_id = dumb.handle;
	atom.hdisplay = 1920;
	atom.vdisplay = 1080;
	atom.active = 1;
	assert(hermes_drm_logic_atomic(&L, &atom) == HERMES_DRM_OK);
	assert(atom.sequence == 1);
	assert(L.active_crtcs == 1);

	assert(hermes_drm_logic_disable(&L, 1) == HERMES_DRM_OK);
	assert(L.active_crtcs == 0);

	/* EDID offline fails closed */
	{
		struct hermes_drm_edid edid;
		struct hermes_drm_prop_get prop;
		unsigned sum = 0;
		int i;

		hermes_drm_logic_init(&L, 0);
		memset(&edid, 0, sizeof(edid));
		edid.connector_id = 1;
		assert(hermes_drm_logic_get_edid(&L, &edid) == HERMES_DRM_E_GSP_OFFLINE);
		assert(edid.size == 0);

		hermes_drm_logic_init(&L, 1);
		assert(hermes_drm_logic_get_edid(&L, &edid) == HERMES_DRM_OK);
		assert(edid.size == 128);
		assert(edid.data[0] == 0x00 && edid.data[1] == 0xff);
		for (i = 0; i < 128; i++)
			sum += edid.data[i];
		assert((sum & 0xff) == 0);

		memset(&prop, 0, sizeof(prop));
		prop.object_id = 1;
		prop.prop_id = HERMES_DRM_PROP_EDID;
		assert(hermes_drm_logic_get_prop(&L, &prop) == HERMES_DRM_OK);
		assert(prop.value == 1);
	}

	printf("test_drm_host: PASS\n");
	return 0;
}
