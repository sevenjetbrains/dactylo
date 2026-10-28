"""Génère `livre.pdf`, PDF de six pages pour les tests e2e de l'import (F-21, F-22).

Chaque page a un en-tête courant (alterné pages paires / impaires), deux lignes de
texte dont la première finit par une césure, et un numéro de page en bas.
Sans dépendance : PDF écrit à la main, police Helvetica en WinAnsiEncoding.
Usage : python tests/e2e/fixtures/make-pdf.py
"""

import pathlib

WORDS = ["un", "deux", "trois", "quatre", "cinq", "six"]


def escape(text: str) -> bytes:
    """Chaîne PDF littérale, encodée en cp1252 (WinAnsi), parenthèses échappées."""
    raw = text.encode("cp1252")
    return raw.replace(b"\\", b"\\\\").replace(b"(", b"\\(").replace(b")", b"\\)")


def page_stream(n: int) -> bytes:
    """Contenu d'une page : une ligne de texte par instruction `Tj`."""
    header = "LE TOUR DU MONDE EN 80 JOURS" if n % 2 == 0 else "CHAPITRE I"
    word = WORDS[n - 1]
    lines = [
        (750, header),
        (700, f"Page {word} : Phileas Fogg était un homme fort exact, qui ne se pres-"),
        (684, f"sait jamais et arrivait toujours à l'heure, page {word}."),
        (60, str(n)),
    ]
    ops = [b"BT /F1 12 Tf"]
    for y, text in lines:
        ops.append(b"1 0 0 1 72 %d Tm (" % y + escape(text) + b") Tj")
    ops.append(b"ET")
    return b"\n".join(ops)


def build() -> bytes:
    """Assemble les objets PDF et la table des références croisées."""
    pages = len(WORDS)
    objects: list[bytes] = []
    # 1 : catalogue, 2 : arbre des pages, 3 : police ; puis page et contenu par page.
    kids = " ".join(f"{4 + 2 * i} 0 R" for i in range(pages))
    objects.append(b"<< /Type /Catalog /Pages 2 0 R >>")
    objects.append(f"<< /Type /Pages /Kids [{kids}] /Count {pages} >>".encode())
    objects.append(
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
    )
    for i in range(pages):
        stream = page_stream(i + 1)
        objects.append(
            f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] "
            f"/Resources << /Font << /F1 3 0 R >> >> /Contents {5 + 2 * i} 0 R >>".encode()
        )
        objects.append(b"<< /Length %d >>\nstream\n" % len(stream) + stream + b"\nendstream")
    out = bytearray(b"%PDF-1.4\n")
    offsets = []
    for number, body in enumerate(objects, start=1):
        offsets.append(len(out))
        out += b"%d 0 obj\n" % number + body + b"\nendobj\n"
    xref = len(out)
    out += b"xref\n0 %d\n0000000000 65535 f \n" % (len(objects) + 1)
    for offset in offsets:
        out += b"%010d 00000 n \n" % offset
    out += b"trailer\n<< /Size %d /Root 1 0 R >>\nstartxref\n%d\n%%%%EOF\n" % (
        len(objects) + 1,
        xref,
    )
    return bytes(out)


if __name__ == "__main__":
    target = pathlib.Path(__file__).with_name("livre.pdf")
    target.write_bytes(build())
    print(target)
