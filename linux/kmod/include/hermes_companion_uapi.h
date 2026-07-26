/* SPDX-License-Identifier: MIT */
/* Shared companion chardev status (modeset / uvm / peermem). */
#ifndef HERMES_COMPANION_UAPI_H
#define HERMES_COMPANION_UAPI_H

#include "hermes_ctl_uapi.h"

#ifndef HERMES_HOST_TEST
/* Same status layout as nvidiactl; distinct ioctl nr for companion surfaces. */
#define HERMES_COMPANION_IOCTL_STATUS \
	_IOR(HERMES_CTL_IOCTL_BASE, 0x20, struct hermes_ctl_status)
#define HERMES_UVM_IOCTL_STATUS HERMES_COMPANION_IOCTL_STATUS
#endif

#endif
