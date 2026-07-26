! Drop-in session surfaces (smi / NVML / CUDA / settings) — phase-gated.
module hermes_dropin
  use hermes_kinds, only: i32, phase_offline, phase_online
  implicit none
  private
  public :: session_t, open_offline_session, promote_online, smi_lists_devices, &
            telemetry_legal, catalog_size, close_session

  type :: session_t
    integer(i32) :: phase = phase_offline
    integer(i32) :: device_count = 0
    logical :: open = .false.
  end type session_t

contains

  pure type(session_t) function open_offline_session(devices) result(s)
    integer(i32), intent(in) :: devices
    s%phase = phase_offline
    s%device_count = max(0_i32, devices)
    s%open = .true.
  end function open_offline_session

  pure type(session_t) function promote_online(s_in) result(s)
    type(session_t), intent(in) :: s_in
    s = s_in
    if (.not. s%open) error stop 'promote: session closed'
    if (s%device_count <= 0) error stop 'promote: no devices'
    s%phase = phase_online
  end function promote_online

  pure logical function smi_lists_devices(s) result(ok)
    type(session_t), intent(in) :: s
    ok = s%open .and. s%device_count > 0
  end function smi_lists_devices

  pure logical function telemetry_legal(s) result(ok)
    type(session_t), intent(in) :: s
    ok = s%open .and. s%phase == phase_online
  end function telemetry_legal

  pure integer(i32) function catalog_size() result(n)
    ! Matches hermes_linux::DROP_IN_PARITY_TARGET / DROP_IN_CATALOG length.
    n = 24
  end function catalog_size

  pure type(session_t) function close_session(s_in) result(s)
    type(session_t), intent(in) :: s_in
    s = s_in
    s%open = .false.
    s%phase = phase_offline
    s%device_count = 0
  end function close_session

end module hermes_dropin
