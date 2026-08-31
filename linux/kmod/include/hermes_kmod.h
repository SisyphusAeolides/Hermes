/* Shared Hermes kmod API — clean-room, fail-closed GSP bring-up. */
#ifndef HERMES_KMOD_H
#define HERMES_KMOD_H

#ifdef HERMES_HOST_TEST
#include <stdbool.h>
#include <stdint.h>
typedef uint8_t u8;
typedef uint16_t u16;
typedef uint32_t u32;
#else
#include <linux/types.h>
#endif

#define HERMES_VENDOR_NVIDIA 0x10deU
#define HERMES_VENDOR_AMD 0x1002U
#define HERMES_VENDOR_INTEL 0x8086U

enum hermes_phase {
	HERMES_PHASE_OFFLINE = 0,
	HERMES_PHASE_PROBED = 1,
	HERMES_PHASE_FIRMWARED = 2,
	HERMES_PHASE_QUEUED = 3,
	HERMES_PHASE_NEGOTIATED = 4,
	HERMES_PHASE_ONLINE = 5,
	HERMES_PHASE_RECOVERING = 6,
	HERMES_PHASE_QUARANTINED = 7,
};

enum hermes_bringup_status {
	HERMES_BRINGUP_OK = 0,
	HERMES_BRINGUP_NOT_NVIDIA = 1,
	HERMES_BRINGUP_PRE_TURING = 2,
	HERMES_BRINGUP_NOT_DISPLAY = 3,
	HERMES_BRINGUP_FIRMWARE = 4,
	HERMES_BRINGUP_ISOLATION = 5,
	HERMES_BRINGUP_INCOMPLETE_EVIDENCE = 6,
	HERMES_BRINGUP_INTERNAL = 7,
	HERMES_BRINGUP_UNSUPPORTED_VENDOR = 8,
};

struct hermes_pci_id {
	u16 vendor;
	u16 device;
	u8 class_code;
	u8 subclass;
};

struct hermes_hw_evidence {
	bool iommu_isolated;
	u32 dma_domain;
	bool wpr_locked;
	bool mailbox_ok;
	bool ready_ok;
	bool firmware_measured;
};

struct hermes_bringup_result {
	enum hermes_bringup_status status;
	enum hermes_phase phase;
	bool online;
	u32 domain_id;
};

bool hermes_is_turing_or_newer(u16 device_id);
struct hermes_bringup_result hermes_run_bringup(const struct hermes_pci_id *id,
						const struct hermes_hw_evidence *ev);
const char *hermes_phase_name(enum hermes_phase phase);

/* Global Online flag published by nvidia.ko for companion modules. */
bool hermes_gsp_is_online(void);
enum hermes_phase hermes_gsp_phase(void);
void hermes_gsp_set_state(bool online, enum hermes_phase phase);

/* When true, HERMES_CTL_IOCTL_SIM_PROMOTE may mint Online with full sim evidence. */
extern bool hermes_allow_sim_promote;

/* Character device surface (/dev/nvidiactl, /dev/nvidia0). */
int hermes_chardev_init(void);
void hermes_chardev_exit(void);

/* Peermem companion: registration authorized only when GSP Online. */
bool hermes_peermem_register_ok(void);

#endif /* HERMES_KMOD_H */
