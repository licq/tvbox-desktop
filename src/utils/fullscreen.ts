export async function enterFullscreen(
  element: HTMLElement | null,
  nativeEnter: () => Promise<void>
) {
  if (element?.requestFullscreen) {
    try {
      await element.requestFullscreen()
      return
    } catch {}
  }
  await nativeEnter()
}

export async function exitFullscreen(
  doc: Document,
  nativeExit: () => Promise<void>
) {
  if (doc.fullscreenElement && doc.exitFullscreen) {
    try {
      await doc.exitFullscreen()
      return
    } catch {}
  }
  await nativeExit()
}
