! Exclusive resource protocol for GSP bring-up (Fortran ownership discipline).
!
! Linear-style resources are handles with a live flag. Transfer subroutines
! require live=.true., set the source dead, and return a new live handle.
! There is no ignite path that skips domain + WPR consumption.
module hermes_resources
  use hermes_kinds, only: i32
  implicit none
  private

  public :: handle_t, match_pci, reject_pci, measure_firmware, discard_firmware, &
            isolate_domain, release_domain, map_bar, unmap_bar, allocate_dma, &
            release_dma, lock_wpr, unlock_wpr, observe_mailbox, observe_ready, &
            admit_features, ignite, shutdown_session, handle_is_live, handle_dead

  type :: handle_t
    integer(i32) :: id = 0
    logical :: live = .false.
  end type handle_t

  integer(i32), save :: next_id = 1

contains

  pure logical function handle_is_live(h) result(ok)
    type(handle_t), intent(in) :: h
    ok = h%live .and. h%id > 0
  end function handle_is_live

  pure type(handle_t) function handle_dead() result(h)
    h = handle_t(0, .false.)
  end function handle_dead

  type(handle_t) function fresh() result(h)
    h%id = next_id
    next_id = next_id + 1
    h%live = .true.
  end function fresh

  subroutine consume(h)
    type(handle_t), intent(inout) :: h
    if (.not. handle_is_live(h)) error stop 'hermes_resources: double-consume'
    h%live = .false.
    h%id = 0
  end subroutine consume

  type(handle_t) function match_pci() result(h)
    h = fresh()
  end function match_pci

  subroutine reject_pci(m)
    type(handle_t), intent(inout) :: m
    call consume(m)
  end subroutine reject_pci

  type(handle_t) function measure_firmware(m) result(seal)
    type(handle_t), intent(inout) :: m
    call consume(m)
    seal = fresh()
  end function measure_firmware

  subroutine discard_firmware(seal)
    type(handle_t), intent(inout) :: seal
    call consume(seal)
  end subroutine discard_firmware

  type(handle_t) function isolate_domain(seal) result(domain)
    type(handle_t), intent(inout) :: seal
    call consume(seal)
    domain = fresh()
  end function isolate_domain

  subroutine release_domain(domain)
    type(handle_t), intent(inout) :: domain
    call consume(domain)
  end subroutine release_domain

  type(handle_t) function map_bar(domain) result(window)
    type(handle_t), intent(inout) :: domain
    call consume(domain)
    window = fresh()
  end function map_bar

  type(handle_t) function unmap_bar(window) result(domain)
    type(handle_t), intent(inout) :: window
    call consume(window)
    domain = fresh()
  end function unmap_bar

  type(handle_t) function allocate_dma(domain) result(region)
    type(handle_t), intent(inout) :: domain
    call consume(domain)
    region = fresh()
  end function allocate_dma

  type(handle_t) function release_dma(region) result(domain)
    type(handle_t), intent(inout) :: region
    call consume(region)
    domain = fresh()
  end function release_dma

  type(handle_t) function lock_wpr(domain) result(wpr)
    type(handle_t), intent(inout) :: domain
    call consume(domain)
    wpr = fresh()
  end function lock_wpr

  type(handle_t) function unlock_wpr(wpr) result(domain)
    type(handle_t), intent(inout) :: wpr
    call consume(wpr)
    domain = fresh()
  end function unlock_wpr

  type(handle_t) function observe_mailbox() result(h)
    h = fresh()
  end function observe_mailbox

  type(handle_t) function observe_ready() result(h)
    h = fresh()
  end function observe_ready

  type(handle_t) function admit_features() result(h)
    h = fresh()
  end function admit_features

  ! Online requires domain + WPR + free evidence. No skip path.
  type(handle_t) function ignite(domain, wpr, mailbox, ready, features) result(session)
    type(handle_t), intent(inout) :: domain, wpr, mailbox, ready, features
    if (.not. handle_is_live(domain)) error stop 'ignite: dead domain'
    if (.not. handle_is_live(wpr)) error stop 'ignite: dead wpr'
    if (.not. handle_is_live(mailbox)) error stop 'ignite: dead mailbox'
    if (.not. handle_is_live(ready)) error stop 'ignite: dead ready'
    if (.not. handle_is_live(features)) error stop 'ignite: dead features'
    call consume(domain)
    call consume(wpr)
    call consume(mailbox)
    call consume(ready)
    call consume(features)
    session = fresh()
  end function ignite

  subroutine shutdown_session(session)
    type(handle_t), intent(inout) :: session
    call consume(session)
  end subroutine shutdown_session

end module hermes_resources
