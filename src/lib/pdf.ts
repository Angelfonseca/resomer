// Limpieza mínima de Markdown a texto legible: jsPDF no renderiza Markdown, y
// para un export de resumen/chat basta con quitar la sintaxis y conservar la
// estructura (encabezados en mayúsculas, viñetas con •).
function markdownToPlain(md: string): string {
  return md
    .replace(/^#{1,6}\s+(.*)$/gm, (_, t) => t.toUpperCase())
    .replace(/\*\*(.*?)\*\*/g, "$1")
    .replace(/\*(.*?)\*/g, "$1")
    .replace(/`([^`]*)`/g, "$1")
    .replace(/^\s*[-*]\s+\[[ x]\]\s*/gim, "• ")
    .replace(/^\s*[-*]\s+/gm, "• ")
    .replace(/\[([^\]]+)\]\([^)]*\)/g, "$1")
    .trim()
}

/**
 * Genera y descarga un PDF simple (texto fluido con saltos de página) a partir
 * de contenido Markdown o texto plano. jsPDF se carga con `import()` dinámico:
 * pesa ~900 KB y no debe entrar en el bundle principal, solo cuando el usuario
 * exporta.
 */
export function downloadPdf(filename: string, title: string, body: string, isMarkdown = true) {
  void import("jspdf")
    .then(({ jsPDF }) => buildAndSavePdf(jsPDF, filename, title, body, isMarkdown))
    .catch((err) => {
      console.error("No se pudo exportar el PDF:", err)
    })
}

function buildAndSavePdf(
  jsPDF: typeof import("jspdf").jsPDF,
  filename: string,
  title: string,
  body: string,
  isMarkdown: boolean
) {
  const doc = new jsPDF({ unit: "pt", format: "a4" })
  const margin = 48
  const pageWidth = doc.internal.pageSize.getWidth()
  const pageHeight = doc.internal.pageSize.getHeight()
  const maxWidth = pageWidth - margin * 2
  let y = margin

  const addLines = (text: string, size: number, bold: boolean, gap: number) => {
    doc.setFont("helvetica", bold ? "bold" : "normal")
    doc.setFontSize(size)
    const lines = doc.splitTextToSize(text, maxWidth) as string[]
    for (const line of lines) {
      if (y + size > pageHeight - margin) {
        doc.addPage()
        y = margin
      }
      doc.text(line, margin, y)
      y += size * 1.35
    }
    y += gap
  }

  addLines(title, 18, true, 10)

  const content = isMarkdown ? markdownToPlain(body) : body
  for (const paragraph of content.split(/\n{2,}/)) {
    const trimmed = paragraph.trim()
    if (!trimmed) continue
    addLines(trimmed, 11, false, 8)
  }

  doc.save(filename.endsWith(".pdf") ? filename : `${filename}.pdf`)
}
