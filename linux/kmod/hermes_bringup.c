// SPDX-License-Identifier: MIT
/*
 * Shared evidence-driven GSP bring-up for Hermes kmod surfaces.
 * Mirrors hermes_gsp::run_bringup gate order and publishes Online only after
 * every required hardware token has been observed.
 */

#ifdef HERMES_HOST_TEST
#include <stdbool.h>
#include <stdint.h>
typedef uint8_t u8;
typedef uint16_t u16;
typedef uint32_t u32;
#define EXPORT_SYMBOL_GPL(sym)
#else
#include <linux/kernel.h>
#include <linux/module.h>
#endif

#include "include/hermes_kmod.h"

bool hermes_is_turing_or_newer(u16 device_id)
{
	if ((device_id >= 0x1e00 && device_id <= 0x1fff) ||
	    (device_id >= 0x2180 && device_id <= 0x21ff))
		return true; /* Turing */
	if ((device_id >= 0x2000 && device_id <= 0x20ff) ||
	    (device_id >= 0x2200 && device_id <= 0x22ff) ||
	    (device_id >= 0x2400 && device_id <= 0x25ff))
		return true; /* Ampere */
	if (device_id >= 0x2300 && device_id <= 0x23ff)
		return true; /* Hopper */
	if (device_id >= 0x2600 && device_id <= 0x28ff)
		return true; /* Ada */
	if (device_id >= 0x2900 && device_id <= 0x2fff)
		return true; /* Blackwell */
	return false;
}
#ifndef HERMES_HOST_TEST
EXPORT_SYMBOL_GPL(hermes_is_turing_or_newer);
#endif

const char *hermes_phase_name(enum hermes_phase phase)
{
	switch (phase) {
	case HERMES_PHASE_OFFLINE:
		return "OFFLINE";
	case HERMES_PHASE_PROBED:
		return "PROBED";
	case HERMES_PHASE_FIRMWARED:
		return "FIRMWARED";
	case HERMES_PHASE_QUEUED:
		return "QUEUED";
	case HERMES_PHASE_NEGOTIATED:
		return "NEGOTIATED";
	case HERMES_PHASE_ONLINE:
		return "ONLINE";
	case HERMES_PHASE_RECOVERING:
		return "RECOVERING";
	case HERMES_PHASE_QUARANTINED:
		return "QUARANTINED";
	default:
		return "UNKNOWN";
	}
}
#ifndef HERMES_HOST_TEST
EXPORT_SYMBOL_GPL(hermes_phase_name);
#endif

struct hermes_bringup_result hermes_run_bringup(const struct hermes_pci_id *id,
						const struct hermes_hw_evidence *ev)
{
	struct hermes_bringup_result r = {
		.status = HERMES_BRINGUP_INTERNAL,
		.phase = HERMES_PHASE_OFFLINE,
		.online = false,
		.domain_id = 0,
	};

	if (!id || !ev) {
		r.status = HERMES_BRINGUP_INTERNAL;
		return r;
	}

	if (id->vendor != HERMES_VENDOR_NVIDIA && id->vendor != HERMES_VENDOR_AMD && id->vendor != HERMES_VENDOR_INTEL) {
		r.status = HERMES_BRINGUP_UNSUPPORTED_VENDOR;
		return r;
	}
	if (id->class_code != 0x03) {
		r.status = HERMES_BRINGUP_NOT_DISPLAY;
		return r;
	}
	if (id->vendor == HERMES_VENDOR_NVIDIA && !hermes_is_turing_or_newer(id->device)) {
		r.status = HERMES_BRINGUP_PRE_TURING;
		return r;
	}

	r.phase = HERMES_PHASE_PROBED;

	if (!ev->firmware_measured) {
		r.status = HERMES_BRINGUP_FIRMWARE;
		return r;
	}
	r.phase = HERMES_PHASE_FIRMWARED;

	if (!ev->iommu_isolated || ev->dma_domain == 0) {
		r.status = HERMES_BRINGUP_ISOLATION;
		return r;
	}
	r.phase = HERMES_PHASE_QUEUED;
	r.domain_id = ev->dma_domain;
	r.phase = HERMES_PHASE_NEGOTIATED;

	if (!ev->wpr_locked || !ev->mailbox_ok || !ev->ready_ok) {
		r.status = HERMES_BRINGUP_INCOMPLETE_EVIDENCE;
		return r;
	}

	r.phase = HERMES_PHASE_ONLINE;
	r.online = true;
	r.status = HERMES_BRINGUP_OK;
	return r;
}
#ifndef HERMES_HOST_TEST
EXPORT_SYMBOL_GPL(hermes_run_bringup);
#endif
