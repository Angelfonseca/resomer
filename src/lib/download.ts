function safeFilename(filename: string): string {
  // Evita que un título con "/" u otros caracteres se interprete como ruta.
  return filename.replace(/[/\\:*?"<>|]+/g, "-").trim() || "descarga.txt"
}

export function downloadTextFile(filename: string, content: string) {
  const blob = new Blob([content], { type: "text/plain;charset=utf-8" })
  const url = URL.createObjectURL(blob)
  const anchor = document.createElement("a")
  anchor.href = url
  anchor.download = safeFilename(filename)
  // El ancla debe estar en el DOM para que el click dispare la descarga de
  // forma fiable en WebKit, y la URL se revoca después (no en el mismo tick).
  document.body.appendChild(anchor)
  anchor.click()
  anchor.remove()
  window.setTimeout(() => URL.revokeObjectURL(url), 0)
}
