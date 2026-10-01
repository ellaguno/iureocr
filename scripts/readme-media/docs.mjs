// Documentos ficticios para las capturas del README (personas, despacho y domicilio
// inventados). make-scans.mjs los convierte en páginas «escaneadas» y mock.ts los
// sirve como si fueran archivos del equipo.

const HERO = {
  es: {
    title: "CONTRATO DE ARRENDAMIENTO",
    sub: "Expediente 214/2026",
    pages: [
      [
        "En la Ciudad de México, a 3 de marzo de 2026, comparecen por una parte la señora Ana Torres Medina, en lo sucesivo la ARRENDADORA, y por la otra Despacho Ruiz & Asociados, S.C., representado por el licenciado Jorge Ruiz Salas, en lo sucesivo el ARRENDATARIO, quienes se reconocen la capacidad legal necesaria para obligarse al tenor de las siguientes:",
        "#CLÁUSULAS",
        "**PRIMERA.** La arrendadora otorga en arrendamiento al arrendatario la oficina 302 del inmueble ubicado en Avenida Ejemplo número 123, destinada exclusivamente a uso de oficinas.",
        "**SEGUNDA.** El arrendatario pagará una renta mensual de $18,500.00 (dieciocho mil quinientos pesos 00/100 M.N.) dentro de los primeros cinco días de cada mes, mediante transferencia bancaria.",
        "**TERCERA.** La vigencia del presente contrato será de doce meses forzosos para ambas partes, contados a partir de la fecha de su firma.",
      ],
      [
        "**CUARTA.** El arrendatario entrega en este acto un depósito en garantía equivalente a un mes de renta, que le será devuelto al término del contrato, previa verificación del estado de la oficina.",
        "**QUINTA.** Queda prohibido al arrendatario subarrendar o ceder los derechos del presente contrato sin consentimiento previo y por escrito de la arrendadora.",
        "**SEXTA.** Los gastos de mantenimiento del edificio correrán por cuenta de la arrendadora; los servicios de luz, internet y teléfono, por cuenta del arrendatario.",
        "**SÉPTIMA.** Cualquiera de las partes podrá dar por terminado el contrato con un aviso por escrito de sesenta días, pagando una pena equivalente a tres meses de renta.",
        "**OCTAVA.** Para la interpretación y cumplimiento del presente contrato, las partes se someten a la jurisdicción de los tribunales de la Ciudad de México.",
      ],
      [
        "Leído que fue el presente contrato y enteradas las partes de su contenido y alcance legal, lo firman por duplicado en la fecha señalada al inicio.",
        "@LA ARRENDADORA|Ana Torres Medina|EL ARRENDATARIO|Despacho Ruiz & Asociados, S.C.",
      ],
    ],
  },
  en: {
    title: "OFFICE LEASE AGREEMENT",
    sub: "File 214/2026",
    pages: [
      [
        "In Mexico City, on March 3, 2026, Ms. Ana Torres Medina, hereinafter the LANDLORD, and Ruiz & Associates Law Firm, represented by attorney Jorge Ruiz Salas, hereinafter the TENANT, both acknowledging their legal capacity, agree to the following:",
        "#CLAUSES",
        "**FIRST.** The landlord leases to the tenant office 302 of the building located at 123 Example Avenue, to be used exclusively as office space.",
        "**SECOND.** The tenant shall pay a monthly rent of $18,500.00 (eighteen thousand five hundred pesos) within the first five days of each month, by bank transfer.",
        "**THIRD.** This agreement shall remain in force for a mandatory term of twelve months for both parties, starting on the date it is signed.",
      ],
      [
        "**FOURTH.** The tenant hereby pays a security deposit equal to one month of rent, to be returned at the end of the agreement after the condition of the office has been checked.",
        "**FIFTH.** The tenant may not sublease the office or assign the rights under this agreement without the prior written consent of the landlord.",
        "**SIXTH.** Building maintenance costs shall be paid by the landlord; electricity, internet and telephone services shall be paid by the tenant.",
        "**SEVENTH.** Either party may terminate this agreement with sixty days written notice, paying a penalty equal to three months of rent.",
        "**EIGHTH.** For the interpretation and performance of this agreement, the parties submit to the jurisdiction of the courts of Mexico City.",
      ],
      [
        "Having read this agreement and being aware of its content and legal scope, the parties sign it in duplicate on the date stated above.",
        "@THE LANDLORD|Ana Torres Medina|THE TENANT|Ruiz & Associates Law Firm",
      ],
    ],
  },
};

// Páginas de relleno: texto genérico para las miniaturas de los demás documentos.
const FILLER = {
  es: [
    "Que por medio del presente escrito y con fundamento en los artículos aplicables del código de procedimientos civiles, se promueve en la vía ordinaria civil en contra de quien resulte responsable, por las prestaciones que más adelante se precisan.",
    "Los hechos en que se funda la presente se narran a continuación, en el orden en que ocurrieron, y se acompañan las pruebas documentales que los acreditan, mismas que se relacionan al final del escrito.",
    "Por lo anteriormente expuesto y fundado, atentamente se solicita tener por presentado el escrito, admitir las pruebas ofrecidas y, en su oportunidad, dictar sentencia favorable.",
  ],
  en: [
    "By means of this filing, and on the basis of the applicable articles of the code of civil procedure, an ordinary civil action is brought against whoever is found responsible, for the claims set out below.",
    "The facts on which this claim is based are described below in the order in which they occurred, together with the documentary evidence that supports them, listed at the end of this filing.",
    "For the reasons stated above, it is respectfully requested that this filing be accepted, the evidence admitted and, in due course, a favorable judgment issued.",
  ],
};

/** Otros documentos de la cola. `kind`: título de la primera página; `pages`: total. */
const OTHERS = {
  es: [
    { id: "demanda", name: "Escrito de demanda 118-2026.pdf", title: "ESCRITO INICIAL DE DEMANDA", pages: 14, size: 6_840_000 },
    { id: "poder", name: "Poder notarial - Ana Torres.pdf", title: "PODER GENERAL PARA PLEITOS Y COBRANZAS", pages: 5, size: 2_310_000 },
    { id: "recibo", name: "Recibo de honorarios.jpg", title: "RECIBO DE HONORARIOS", pages: 1, size: 1_120_000, image: true },
    { id: "dictamen", name: "Dictamen pericial.pdf", title: "DICTAMEN PERICIAL EN MATERIA CONTABLE", pages: 9, size: 840_000 },
  ],
  en: [
    { id: "demanda", name: "Complaint 118-2026.pdf", title: "INITIAL COMPLAINT", pages: 14, size: 6_840_000 },
    { id: "poder", name: "Power of attorney - Ana Torres.pdf", title: "GENERAL POWER OF ATTORNEY", pages: 5, size: 2_310_000 },
    { id: "recibo", name: "Fee receipt.jpg", title: "RECEIPT FOR LEGAL FEES", pages: 1, size: 1_120_000, image: true },
    { id: "dictamen", name: "Expert report.pdf", title: "EXPERT ACCOUNTING REPORT", pages: 9, size: 840_000 },
  ],
};

export const HERO_NAME = { es: "Contrato de arrendamiento - Ruiz & Asociados.pdf", en: "Lease agreement - Ruiz & Associates.pdf" };
export const DIR = { es: "/home/demo/Documentos/Expedientes", en: "/home/demo/Documents/Case files" };

export { HERO, FILLER, OTHERS };
