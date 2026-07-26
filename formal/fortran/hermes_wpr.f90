! WPR2 plan + SEC2 Booter mailbox (exclusive plan/mailbox handles).
module hermes_wpr
  use hermes_kinds, only: i32
  use hermes_resources, only: handle_t, handle_is_live
  implicit none
  private
  public :: observe_framebuffer, observe_dma, observe_boot_offsets, build_plan, &
            discard_plan, submit_booter_load, complete_booter, reject_booter, &
            observe_wpr2_active

  integer(i32), save :: next_id = 3000

contains

  type(handle_t) function mint() result(h)
    h%id = next_id
    next_id = next_id + 1
    h%live = .true.
  end function mint

  subroutine kill(h)
    type(handle_t), intent(inout) :: h
    if (.not. handle_is_live(h)) error stop 'wpr: double-consume'
    h%live = .false.
    h%id = 0
  end subroutine kill

  type(handle_t) function observe_framebuffer() result(h)
    h = mint()
  end function observe_framebuffer

  type(handle_t) function observe_dma() result(h)
    h = mint()
  end function observe_dma

  type(handle_t) function observe_boot_offsets() result(h)
    h = mint()
  end function observe_boot_offsets

  type(handle_t) function build_plan(fb, dma, boot) result(plan)
    type(handle_t), intent(inout) :: fb, dma, boot
    call kill(fb)
    call kill(dma)
    call kill(boot)
    plan = mint()
  end function build_plan

  subroutine discard_plan(plan)
    type(handle_t), intent(inout) :: plan
    call kill(plan)
  end subroutine discard_plan

  type(handle_t) function submit_booter_load(plan) result(mailbox)
    type(handle_t), intent(inout) :: plan
    call kill(plan)
    mailbox = mint()
  end function submit_booter_load

  subroutine complete_booter(mailbox, wpr2)
    type(handle_t), intent(inout) :: mailbox, wpr2
    call kill(mailbox)
    call kill(wpr2)
  end subroutine complete_booter

  subroutine reject_booter(mailbox)
    type(handle_t), intent(inout) :: mailbox
    call kill(mailbox)
  end subroutine reject_booter

  type(handle_t) function observe_wpr2_active() result(h)
    h = mint()
  end function observe_wpr2_active

end module hermes_wpr
