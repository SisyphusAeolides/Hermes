! Live GPU handle lifecycle: fault/contain/release — no free Online skip.
module hermes_fail_closed
  use hermes_kinds, only: i32, phase_offline, phase_online, phase_quarantined
  use hermes_resources, only: handle_t, handle_is_live
  implicit none
  private
  public :: probe_gpu, activate_gpu, fault_gpu, recover_gpu, contain_gpu, &
            release_gpu, gpu_phase

  integer(i32), save :: next_id = 2000

contains

  type(handle_t) function mint() result(h)
    h%id = next_id
    next_id = next_id + 1
    h%live = .true.
  end function mint

  subroutine kill(h)
    type(handle_t), intent(inout) :: h
    if (.not. handle_is_live(h)) error stop 'fail_closed: double-consume'
    h%live = .false.
    h%id = 0
  end subroutine kill

  type(handle_t) function probe_gpu() result(p)
    p = mint()
  end function probe_gpu

  type(handle_t) function activate_gpu(probe) result(live)
    type(handle_t), intent(inout) :: probe
    call kill(probe)
    live = mint()
  end function activate_gpu

  type(handle_t) function fault_gpu(gpu) result(q)
    type(handle_t), intent(inout) :: gpu
    call kill(gpu)
    q = mint()
  end function fault_gpu

  type(handle_t) function recover_gpu(q) result(live)
    type(handle_t), intent(inout) :: q
    call kill(q)
    live = mint()
  end function recover_gpu

  type(handle_t) function contain_gpu(q) result(off)
    type(handle_t), intent(inout) :: q
    call kill(q)
    off = mint()
  end function contain_gpu

  type(handle_t) function release_gpu(gpu) result(off)
    type(handle_t), intent(inout) :: gpu
    call kill(gpu)
    off = mint()
  end function release_gpu

  pure integer(i32) function gpu_phase(live_gpu, quarantined) result(p)
    logical, intent(in) :: live_gpu, quarantined
    if (quarantined) then
      p = phase_quarantined
    else if (live_gpu) then
      p = phase_online
    else
      p = phase_offline
    end if
  end function gpu_phase

end module hermes_fail_closed
