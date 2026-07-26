! Five-file Turing bootstrap bundle (SEC2 / GSP BL / Booter / GSP-RM).
module hermes_bootstrap
  use hermes_kinds, only: i32
  use hermes_resources, only: handle_t, handle_is_live
  implicit none
  private
  public :: measure_sec2, measure_gsp_boot, measure_booter_load, &
            measure_booter_unload, measure_gsp_rm, verify_bundle, attach_rm, &
            release_bundle

  integer(i32), save :: next_id = 4000

contains

  type(handle_t) function mint() result(h)
    h%id = next_id
    next_id = next_id + 1
    h%live = .true.
  end function mint

  subroutine kill(h)
    type(handle_t), intent(inout) :: h
    if (.not. handle_is_live(h)) error stop 'bootstrap: double-consume'
    h%live = .false.
    h%id = 0
  end subroutine kill

  type(handle_t) function measure_sec2() result(h)
    h = mint()
  end function measure_sec2

  type(handle_t) function measure_gsp_boot() result(h)
    h = mint()
  end function measure_gsp_boot

  type(handle_t) function measure_booter_load() result(h)
    h = mint()
  end function measure_booter_load

  type(handle_t) function measure_booter_unload() result(h)
    h = mint()
  end function measure_booter_unload

  type(handle_t) function measure_gsp_rm() result(h)
    h = mint()
  end function measure_gsp_rm

  type(handle_t) function verify_bundle(sec2, gsp_boot, load, unload) result(bundle)
    type(handle_t), intent(inout) :: sec2, gsp_boot, load, unload
    call kill(sec2)
    call kill(gsp_boot)
    call kill(load)
    call kill(unload)
    bundle = mint()
  end function verify_bundle

  type(handle_t) function attach_rm(bundle, rm) result(out)
    type(handle_t), intent(inout) :: bundle, rm
    call kill(bundle)
    call kill(rm)
    out = mint()
  end function attach_rm

  subroutine release_bundle(bundle)
    type(handle_t), intent(inout) :: bundle
    call kill(bundle)
  end subroutine release_bundle

end module hermes_bootstrap
