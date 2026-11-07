import type { Optimization } from "@/types";

/**
 * Preview catalogue for the M0 shell. The authoritative list will come from
 * `wexo-optimizer` in M2; IDs here are the same ones the Rust registry will use.
 */
export const sampleOptimizations: Optimization[] = [
  {
    id: "disable-sysmain",
    category: "services",
    title: "Deshabilitar SysMain (Superfetch)",
    description:
      "Detiene el servicio que precarga aplicaciones en memoria y reduce la actividad de E/S en segundo plano.",
    rationale:
      "En un SSD/NVMe el acceso aleatorio es de microsegundos. La precarga aporta poco y consume ciclos de disco y RAM.",
    risk: "low",
    requiresReboot: false,
    appliesTo: {
      osMinBuild: "19044",
      hardwareFilters: ["disk.kind == 'nvme' || disk.kind == 'ssd'"],
      conflictsWith: [],
    },
    snapshot: { registryKeys: [], services: ["SysMain"], files: [] },
    available: false,
  },
  {
    id: "disable-diagtrack",
    category: "privacy",
    title: "Deshabilitar telemetría (DiagTrack)",
    description:
      "Detiene el servicio de telemetría de diagnóstico y sus tareas programadas asociadas.",
    rationale:
      "Windows recopila datos de diagnóstico de forma continua. Desactivarlo reduce E/S y actividad de red no esencial.",
    risk: "low",
    requiresReboot: false,
    appliesTo: {
      osMinBuild: "19044",
      hardwareFilters: [],
      conflictsWith: [],
    },
    snapshot: { registryKeys: [], services: ["DiagTrack", "dmwappushservice"], files: [] },
    available: false,
  },
  {
    id: "power-plan-ultimate",
    category: "power",
    title: "Plan de energía de alto rendimiento",
    description:
      "Activa el plan Ultimate Performance y elimina la latencia extra de los estados de ahorro en escritorio.",
    rationale:
      "En equipos de sobremesa, el ahorro de energía no aporta beneficio y añade micro-latencias al cambio de frecuencia.",
    risk: "low",
    requiresReboot: false,
    appliesTo: {
      osMinBuild: "19044",
      hardwareFilters: ["chassis == 'desktop'"],
      conflictsWith: [],
    },
    snapshot: {
      registryKeys: ["HKLM\\SYSTEM\\CurrentControlSet\\Control\\Power"],
      services: [],
      files: [],
    },
    available: false,
  },
  {
    id: "tcp-autotuning",
    category: "network",
    title: "Optimizar autotuning de TCP",
    description:
      "Ajusta la ventana de recepción TCP y el RSS para adaptarse al ancho de banda real del adaptador.",
    rationale:
      "El valor por defecto de Windows puede limitar el throughput en conexiones de alta latencia o enlaces superiores a 1 Gbps.",
    risk: "low",
    requiresReboot: false,
    appliesTo: {
      osMinBuild: "19044",
      hardwareFilters: [],
      conflictsWith: [],
    },
    snapshot: {
      registryKeys: ["HKLM\\SYSTEM\\CurrentControlSet\\Services\\Tcpip\\Parameters"],
      services: [],
      files: [],
    },
    available: false,
  },
  {
    id: "enable-hags",
    category: "gpu",
    title: "Programación de GPU acelerada por hardware",
    description:
      "Activa HAGS para que la GPU gestione su propia cola de trabajo con menor latencia de CPU.",
    rationale:
      "Reduce la intervención de la CPU en la planificación de trabajos gráficos; requiere driver y SO compatibles.",
    risk: "low",
    requiresReboot: true,
    appliesTo: {
      osMinBuild: "19044",
      hardwareFilters: ["gpu.driver_model == 'wddm2'"],
      conflictsWith: [],
    },
    snapshot: {
      registryKeys: ["HKLM\\SYSTEM\\CurrentControlSet\\Control\\GraphicsDrivers"],
      services: [],
      files: [],
    },
    available: false,
  },
  {
    id: "reduce-visual-effects",
    category: "visuals",
    title: "Reducir efectos visuales",
    description:
      "Desactiva animaciones, sombras y transparencias manteniendo la legibilidad y el contraste.",
    rationale:
      "En equipos con GPU integrada o batería, los efectos visuales consumen recursos sin mejorar la productividad.",
    risk: "low",
    requiresReboot: false,
    appliesTo: {
      osMinBuild: "19044",
      hardwareFilters: [],
      conflictsWith: [],
    },
    snapshot: {
      registryKeys: ["HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\VisualEffects"],
      services: [],
      files: [],
    },
    available: false,
  },
  {
    id: "disable-startup-bloat",
    category: "startup",
    title: "Revisar aplicaciones de inicio",
    description:
      "Deshabilita entradas de inicio innecesarias detectadas por impacto medido de CPU, disco y tiempo de arranque.",
    rationale:
      "Cada aplicación de inicio añade latencia al login. Solo se desactivan entradas sin impacto funcional demostrado.",
    risk: "low",
    requiresReboot: false,
    appliesTo: {
      osMinBuild: "19044",
      hardwareFilters: [],
      conflictsWith: [],
    },
    snapshot: {
      registryKeys: ["HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run"],
      services: [],
      files: [],
    },
    available: false,
  },
  {
    id: "ntfs-last-access",
    category: "file-system",
    title: "Desactivar actualización de último acceso",
    description:
      "Evita que NTFS escriba la marca de tiempo de último acceso en cada lectura de archivo.",
    rationale:
      "Reduce escrituras innecesarias en cargas de trabajo con muchos archivos pequeños, como compilaciones o juegos.",
    risk: "low",
    requiresReboot: true,
    appliesTo: {
      osMinBuild: "19044",
      hardwareFilters: [],
      conflictsWith: [],
    },
    snapshot: {
      registryKeys: ["HKLM\\SYSTEM\\CurrentControlSet\\Control\\FileSystem"],
      services: [],
      files: [],
    },
    available: false,
  },
];
