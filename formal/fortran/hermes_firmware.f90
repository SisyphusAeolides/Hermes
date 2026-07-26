! Measured GSP-RM admission: family + digest + ELF → exclusive seal.
module hermes_firmware
  use hermes_kinds, only: i32
  use hermes_resources, only: handle_t, handle_is_live
  implicit none
  private
  public :: open_image, classify_family, hash_image, parse_elf, seal_firmware, &
            reject_image, discard_seal, family_tu10x, family_ga10x

  integer(i32), parameter :: family_tu10x = 1
  integer(i32), parameter :: family_ga10x = 2

  integer(i32), save :: next_id = 5000

contains

  type(handle_t) function mint() result(h)
    h%id = next_id
    next_id = next_id + 1
    h%live = .true.
  end function mint

  subroutine kill(h)
    type(handle_t), intent(inout) :: h
    if (.not. handle_is_live(h)) error stop 'firmware: double-consume'
    h%live = .false.
    h%id = 0
  end subroutine kill

  type(handle_t) function open_image() result(h)
    h = mint()
  end function open_image

  integer(i32) function classify_family(image) result(fam)
    type(handle_t), intent(in) :: image
    if (.not. handle_is_live(image)) error stop 'classify_family: dead image'
    fam = family_tu10x
  end function classify_family

  type(handle_t) function hash_image(image) result(digest)
    type(handle_t), intent(in) :: image
    if (.not. handle_is_live(image)) error stop 'hash_image: dead image'
    digest = mint()
  end function hash_image

  type(handle_t) function parse_elf(image) result(elf)
    type(handle_t), intent(in) :: image
    if (.not. handle_is_live(image)) error stop 'parse_elf: dead image'
    elf = mint()
  end function parse_elf

  type(handle_t) function seal_firmware(family, digest, elf, image) result(seal)
    integer(i32), intent(in) :: family
    type(handle_t), intent(inout) :: digest, elf, image
    if (family /= family_tu10x .and. family /= family_ga10x) &
      error stop 'seal: unknown family'
    call kill(digest)
    call kill(elf)
    call kill(image)
    seal = mint()
  end function seal_firmware

  subroutine reject_image(image)
    type(handle_t), intent(inout) :: image
    call kill(image)
  end subroutine reject_image

  subroutine discard_seal(seal)
    type(handle_t), intent(inout) :: seal
    call kill(seal)
  end subroutine discard_seal

end module hermes_firmware
