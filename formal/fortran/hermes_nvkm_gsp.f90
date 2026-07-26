! Nouveau-shaped GSP firmware bundle (clean-room exclusive handles).
module hermes_nvkm_gsp
  use hermes_kinds, only: i32
  use hermes_resources, only: handle_t, handle_is_live
  implicit none
  private
  public :: chip_id, version_tag, load_booter_load, load_booter_unload, &
            load_bootloader, load_fmc, load_gsp_rm, assemble_booter, &
            assemble_fmc, hermes_ignite_nvkm, release_online_nvkm

  integer(i32), save :: next_id = 10000

contains

  type(handle_t) function mint() result(h)
    h%id = next_id
    next_id = next_id + 1
    h%live = .true.
  end function mint

  subroutine kill(h)
    type(handle_t), intent(inout) :: h
    if (.not. handle_is_live(h)) error stop 'nvkm: double-consume'
    h%live = .false.
    h%id = 0
  end subroutine kill

  pure integer(i32) function chip_id() result(c)
    c = int(z'1fb9', i32)  ! sample Turing
  end function chip_id

  pure integer(i32) function version_tag() result(v)
    v = 6104303
  end function version_tag

  type(handle_t) function load_booter_load() result(h)
    h = mint()
  end function load_booter_load

  type(handle_t) function load_booter_unload() result(h)
    h = mint()
  end function load_booter_unload

  type(handle_t) function load_bootloader() result(h)
    h = mint()
  end function load_bootloader

  type(handle_t) function load_fmc() result(h)
    h = mint()
  end function load_fmc

  type(handle_t) function load_gsp_rm() result(h)
    h = mint()
  end function load_gsp_rm

  type(handle_t) function assemble_booter(load, unload, bl, rm) result(bundle)
    type(handle_t), intent(inout) :: load, unload, bl, rm
    call kill(load)
    call kill(unload)
    call kill(bl)
    call kill(rm)
    bundle = mint()
  end function assemble_booter

  type(handle_t) function assemble_fmc(fmc, bl, rm) result(bundle)
    type(handle_t), intent(inout) :: fmc, bl, rm
    call kill(fmc)
    call kill(bl)
    call kill(rm)
    bundle = mint()
  end function assemble_fmc

  type(handle_t) function hermes_ignite_nvkm(bundle) result(online)
    type(handle_t), intent(inout) :: bundle
    call kill(bundle)
    online = mint()
  end function hermes_ignite_nvkm

  subroutine release_online_nvkm(session)
    type(handle_t), intent(inout) :: session
    call kill(session)
  end subroutine release_online_nvkm

end module hermes_nvkm_gsp
