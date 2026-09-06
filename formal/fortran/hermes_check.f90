! Executable formal gate: exercise exclusive-handle protocols; abort on failure.
program hermes_check
  use hermes_kinds, only: i32, i64, phase_online, phase_label
  use hermes_resources, only: handle_t, handle_is_live, match_pci, measure_firmware, &
       isolate_domain, lock_wpr, observe_mailbox, observe_ready, admit_features, &
       ignite, shutdown_session, map_bar, unmap_bar
  use hermes_rings, only: arm_command, arm_event, pair_rings, take_slot, &
       retire_slot, disarm
  use hermes_lifecycle, only: probe_gpu, activate_gpu, fault_gpu, contain_gpu, &
       release_gpu, gpu_phase
  use hermes_wpr, only: observe_framebuffer, observe_dma, observe_boot_offsets, &
       build_plan, submit_booter_load, observe_wpr2_active, complete_booter
  use hermes_bootstrap, only: measure_sec2, measure_gsp_boot, measure_booter_load, &
       measure_booter_unload, measure_gsp_rm, verify_bundle, attach_rm, release_bundle
  use hermes_firmware, only: open_image, classify_family, hash_image, parse_elf, &
       seal_firmware, discard_seal
  use hermes_mailbox, only: require_gsp_mb, open_mailbox, post_hello, observe_ready_resp
  use hermes_host_gate, only: host_facts_t, observe_facts, may_claim_online, &
       mint_authority, drop_authority
  use hermes_dropin, only: session_t, open_offline_session, promote_online, &
       smi_lists_devices, telemetry_legal, catalog_size, close_session
  use hermes_drm_kms, only: require_gsp_drm, open_crtc, open_plane, bind_framebuffer, &
       atomic_apply, close_modeset
  use hermes_cccl, only: require_gsp_cuda, open_driver, create_context, alloc_device, &
       free_device, close_context, close_driver
  use hermes_nvkm_gsp, only: load_booter_load, load_booter_unload, load_bootloader, &
       load_gsp_rm, assemble_booter, hermes_ignite_nvkm, release_online_nvkm
  implicit none

  type(handle_t) :: pci, seal, domain, wpr, mb, ready, feat, session
  type(handle_t) :: bar, cmd, evt, transport, slot
  type(handle_t) :: probe, live, q, offline
  type(handle_t) :: fb, dma, boot, plan, booter_mb, wpr2
  type(handle_t) :: sec2, gbl, bl, bu, rm, bundle
  type(handle_t) :: img, dig, elf, fseal
  type(handle_t) :: gsp, mbox, resp, auth
  type(handle_t) :: crtc, plane, modeset, drv, ctx, dbuf
  type(handle_t) :: nload, nunload, nbl, nrm, nbundle, nonline
  type(host_facts_t) :: facts
  type(session_t) :: smi
  integer(i32) :: fam, phase
  integer(i64) :: nbytes

  print *, 'hermes_check: Fortran formal gate'

  ! Resources → Online session
  pci = match_pci()
  seal = measure_firmware(pci)
  domain = isolate_domain(seal)
  bar = map_bar(domain)
  domain = unmap_bar(bar)
  wpr = lock_wpr(domain)
  ! Need a second domain path for ignite: rebuild domain after unlock simulation.
  ! ignite consumes domain+wpr; build domain again via firmware path.
  pci = match_pci()
  seal = measure_firmware(pci)
  domain = isolate_domain(seal)
  mb = observe_mailbox()
  ready = observe_ready()
  feat = admit_features()
  session = ignite(domain, wpr, mb, ready, feat)
  if (.not. handle_is_live(session)) error stop 'session not live'
  call shutdown_session(session)

  ! Rings
  cmd = arm_command(64_i32)
  evt = arm_event(64_i32)
  transport = pair_rings(cmd, evt)
  slot = take_slot(transport)
  transport = retire_slot(slot)
  call disarm(transport)

  ! Evidence-tracked lifecycle
  probe = probe_gpu()
  live = activate_gpu(probe)
  phase = gpu_phase(.true., .false.)
  if (phase /= phase_online) error stop 'expected ONLINE phase'
  q = fault_gpu(live)
  offline = contain_gpu(q)
  if (.not. handle_is_live(offline)) error stop 'contain failed'

  ! WPR plan
  fb = observe_framebuffer()
  dma = observe_dma()
  boot = observe_boot_offsets()
  plan = build_plan(fb, dma, boot)
  booter_mb = submit_booter_load(plan)
  wpr2 = observe_wpr2_active()
  call complete_booter(booter_mb, wpr2)

  ! Bootstrap bundle
  sec2 = measure_sec2()
  gbl = measure_gsp_boot()
  bl = measure_booter_load()
  bu = measure_booter_unload()
  rm = measure_gsp_rm()
  bundle = verify_bundle(sec2, gbl, bl, bu)
  bundle = attach_rm(bundle, rm)
  call release_bundle(bundle)

  ! Firmware seal
  img = open_image()
  fam = classify_family(img)
  dig = hash_image(img)
  elf = parse_elf(img)
  fseal = seal_firmware(fam, dig, elf, img)
  call discard_seal(fseal)

  ! Mailbox
  gsp = require_gsp_mb(.true.)
  mbox = open_mailbox(gsp)
  resp = post_hello(mbox)
  if (.not. observe_ready_resp(resp)) error stop 'mailbox not ready'

  ! Host gate
  facts = observe_facts(.true., .false., .true., .true.)
  if (.not. may_claim_online(facts)) error stop 'host gate should pass'
  auth = mint_authority(facts)
  call drop_authority(auth)
  facts = observe_facts(.false., .true., .false., .false.)
  if (may_claim_online(facts)) error stop 'host gate should fail'

  ! Drop-in session
  smi = open_offline_session(1_i32)
  if (.not. smi_lists_devices(smi)) error stop 'smi should list'
  if (telemetry_legal(smi)) error stop 'telemetry illegal offline'
  smi = promote_online(smi)
  if (.not. telemetry_legal(smi)) error stop 'telemetry legal online'
  if (catalog_size() < 15_i32) error stop 'catalog too small'
  smi = close_session(smi)

  ! DRM
  gsp = require_gsp_drm(.true.)
  crtc = open_crtc(gsp)
  plane = open_plane(gsp)
  plane = bind_framebuffer(plane, 1_i32)
  modeset = atomic_apply(crtc, plane, gsp)
  call close_modeset(modeset)

  ! CCCL/CUDA
  gsp = require_gsp_cuda(.true.)
  drv = open_driver(gsp)
  ctx = create_context(drv)
  nbytes = 1024_i64
  dbuf = alloc_device(ctx, nbytes)
  ctx = free_device(dbuf)
  drv = close_context(ctx)
  call close_driver(drv)

  ! Nvkm bundle
  nload = load_booter_load()
  nunload = load_booter_unload()
  nbl = load_bootloader()
  nrm = load_gsp_rm()
  nbundle = assemble_booter(nload, nunload, nbl, nrm)
  nonline = hermes_ignite_nvkm(nbundle)
  call release_online_nvkm(nonline)

  print *, 'hermes_check: PASS (all Fortran formal modules)'
  print *, 'phase label sample: ', trim(phase_label(phase_online))
end program hermes_check
