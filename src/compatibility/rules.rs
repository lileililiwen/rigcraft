use super::*;

fn cpu_socket(parts: &BuildParts) -> Option<RuleResult> {
    let (cpu, board) = (parts.cpu.as_ref()?, parts.motherboard.as_ref()?);
    let evidence = vec![
        field_evidence(cpu, "socket"),
        field_evidence(board, "socket"),
    ];
    let (cpu_socket, board_socket) = (text(&cpu.record, "socket"), text(&board.record, "socket"));
    let (status, explanation) = match (cpu_socket, board_socket) {
        (Some(a), Some(b)) if a.eq_ignore_ascii_case(b) => (
            Verdict::Compatible,
            format!("CPU and motherboard both declare Socket {a}."),
        ),
        (Some(a), Some(b)) => (
            Verdict::Incompatible,
            format!("CPU declares Socket {a} but the motherboard declares Socket {b}."),
        ),
        _ => (
            Verdict::Unknown,
            "The CPU Socket or motherboard Socket is not recorded.".to_string(),
        ),
    };
    Some(rule(
        "cpu-socket",
        "CPU and motherboard Socket",
        "cpu",
        status,
        vec![part_ref(cpu), part_ref(board)],
        evidence.clone(),
        explanation,
        (status == Verdict::Unknown).then(|| {
            unknown_follow_up(
                &evidence,
                "Confirm the CPU Socket and the motherboard Socket from the manufacturer's specification.",
            )
        }),
        "Exact Socket comparison after trimming; no alias inference and no catalog lookup.",
    ))
}

fn cpu_bios(parts: &BuildParts) -> Option<RuleResult> {
    let (cpu, board) = (parts.cpu.as_ref()?, parts.motherboard.as_ref()?);
    let mut evidence = vec![model_evidence(cpu), field_evidence(board, "bios_supports")];
    let supported = text(&board.record, "bios_supports");
    let model = if cpu.record.model_normalized.is_empty() {
        cpu.record.model_raw.as_str()
    } else {
        cpu.record.model_normalized.as_str()
    };
    let (status, explanation, follow_up) = match supported {
        None => {
            evidence.push(field_evidence(board, "bios"));
            (
                Verdict::Unknown,
                format!("No sourced BIOS support fact is recorded for {model}."),
                Some(format!(
                    "Confirm the motherboard's installed BIOS version supports {model}."
                )),
            )
        }
        Some(value) => {
            let haystack = value.to_lowercase();
            let matches_model = haystack.contains(&model.to_lowercase())
                || haystack.contains(&cpu.record.sku.to_lowercase());
            if matches_model {
                (
                    Verdict::Compatible,
                    format!("The motherboard declares BIOS support for {model}."),
                    None,
                )
            } else {
                (
                    Verdict::Unknown,
                    format!(
                        "The motherboard BIOS support fact names `{value}`, which does not name {model}."
                    ),
                    Some(format!(
                        "Confirm the installed BIOS covers {model}; the recorded fact only names `{value}`."
                    )),
                )
            }
        }
    };
    Some(rule(
        "cpu-bios",
        "BIOS support evidence",
        "cpu",
        status,
        vec![part_ref(cpu), part_ref(board)],
        evidence,
        explanation,
        follow_up,
        "Matches the recorded BIOS support fact against the CPU model and SKU; absence is unknown, not a yes.",
    ))
}

fn ram_type(parts: &BuildParts) -> Option<RuleResult> {
    let board = parts.motherboard.as_ref()?;
    if parts.ram.is_empty() {
        return None;
    }
    let mut evidence = vec![field_evidence(board, "memory_type")];
    let mut status = Verdict::Compatible;
    let mut conflicts = Vec::new();
    for ram in &parts.ram {
        evidence.push(field_evidence(ram, "memory_type"));
        match (
            text(&board.record, "memory_type"),
            text(&ram.record, "memory_type"),
        ) {
            (Some(board_type), Some(ram_type)) => {
                if board_type.eq_ignore_ascii_case(ram_type) {
                    // compatible
                } else {
                    status = status.merge(Verdict::Incompatible);
                    conflicts.push(format!(
                        "{} declares {ram_type} while the motherboard declares {board_type}.",
                        ram.record.sku
                    ));
                }
            }
            _ => status = status.merge(Verdict::Unknown),
        }
    }
    let explanation = if conflicts.is_empty() {
        "Every selected memory kit matches the motherboard memory type.".to_string()
    } else {
        conflicts.join(" ")
    };
    Some(rule(
        "ram-type",
        "Memory type",
        "memory",
        status,
        std::iter::once(part_ref(board))
            .chain(parts.ram.iter().map(part_ref))
            .collect(),
        evidence.clone(),
        explanation,
        (status == Verdict::Unknown)
            .then(|| unknown_follow_up(&evidence, "Confirm the motherboard and memory type.")),
        "Exact memory-type comparison; DDR generations are not treated as interchangeable.",
    ))
}

fn ram_capacity(parts: &BuildParts) -> Option<RuleResult> {
    let board = parts.motherboard.as_ref()?;
    if parts.ram.is_empty() {
        return None;
    }
    let mut evidence = vec![field_evidence(board, "max_memory")];
    let max = number(&board.record, "max_memory");
    let mut total = 0.0;
    let mut complete = true;
    for ram in &parts.ram {
        evidence.push(field_evidence(ram, "capacity"));
        match number(&ram.record, "capacity") {
            Some(capacity) => total += capacity * ram.quantity as f64,
            None => complete = false,
        }
    }
    let (status, explanation, follow_up) = match (max, complete) {
        (Some(max), true) if total > max => (
            Verdict::Incompatible,
            format!("Selected memory totals {total} GB, above the board's {max} GB maximum."),
            None,
        ),
        (Some(max), true) => (
            Verdict::Compatible,
            format!("Selected memory totals {total} GB within the board's {max} GB maximum."),
            None,
        ),
        _ => (
            Verdict::Unknown,
            "The motherboard maximum memory or a selected memory capacity is missing.".to_string(),
            Some(unknown_follow_up(
                &evidence,
                "Confirm the motherboard maximum memory and each memory kit capacity.",
            )),
        ),
    };
    Some(rule(
        "ram-capacity",
        "Memory capacity",
        "memory",
        status,
        std::iter::once(part_ref(board))
            .chain(parts.ram.iter().map(part_ref))
            .collect(),
        evidence,
        explanation,
        follow_up,
        "Sums each kit capacity times its selected quantity and compares it with the recorded board maximum.",
    ))
}

fn ram_slots(parts: &BuildParts) -> Option<RuleResult> {
    let board = parts.motherboard.as_ref()?;
    if parts.ram.is_empty() {
        return None;
    }
    let mut evidence = vec![field_evidence(board, "ram_slots")];
    let slots = number(&board.record, "ram_slots");
    let mut modules = 0.0;
    let mut complete = true;
    for ram in &parts.ram {
        evidence.push(field_evidence(ram, "modules"));
        match number(&ram.record, "modules") {
            Some(count) => modules += count * ram.quantity as f64,
            None => complete = false,
        }
    }
    let (status, explanation, follow_up) = match (slots, complete) {
        (Some(slots), true) if modules > slots => (
            Verdict::Incompatible,
            format!(
                "Selected memory needs {modules} modules but the board provides {slots} slots."
            ),
            None,
        ),
        (Some(slots), true) => (
            Verdict::Compatible,
            format!("Selected memory needs {modules} modules within the board's {slots} slots."),
            None,
        ),
        _ => (
            Verdict::Unknown,
            "The board slot count or a memory kit module count is missing.".to_string(),
            Some(unknown_follow_up(
                &evidence,
                "Confirm the motherboard DIMM slot count and how many modules each kit contains.",
            )),
        ),
    };
    Some(rule(
        "ram-slots",
        "Memory slots",
        "memory",
        status,
        std::iter::once(part_ref(board))
            .chain(parts.ram.iter().map(part_ref))
            .collect(),
        evidence,
        explanation,
        follow_up,
        "Multiplies modules per kit by selected quantity and compares it with the recorded slot count.",
    ))
}

fn pcie_generation(value: &str) -> Option<u32> {
    // Read the first number in the value, so the manufacturer's own wording
    // ("PCIe 4.0 x16", "PCIe5", "5.0") yields the generation rather than every
    // digit in the string.
    let digits: String = value
        .chars()
        .skip_while(|ch| !ch.is_ascii_digit())
        .take_while(|ch| ch.is_ascii_digit())
        .collect();
    digits.parse::<u32>().ok()
}

fn gpu_interface(parts: &BuildParts) -> Option<RuleResult> {
    let (gpu, board) = (parts.gpu.as_ref()?, parts.motherboard.as_ref()?);
    let evidence = vec![
        field_evidence(gpu, "interface"),
        field_evidence(board, "pcie_version"),
    ];
    let (gpu_gen, board_gen) = (
        text(&gpu.record, "interface").and_then(pcie_generation),
        text(&board.record, "pcie_version").and_then(pcie_generation),
    );
    let (status, explanation, follow_up) = match (gpu_gen, board_gen) {
        (Some(gpu_gen), Some(board_gen)) if gpu_gen <= board_gen => (
            Verdict::Compatible,
            format!("Graphics card PCIe generation {gpu_gen} fits the board slot generation {board_gen}."),
            None,
        ),
        (Some(gpu_gen), Some(board_gen)) => (
            Verdict::Incompatible,
            format!("Graphics card needs PCIe generation {gpu_gen} but the board provides {board_gen}."),
            None,
        ),
        _ => (
            Verdict::Unknown,
            "The graphics interface or board PCIe generation is missing.".to_string(),
            Some(unknown_follow_up(
                &evidence,
                "Confirm the graphics-card interface and the motherboard PCIe slot generation.",
            )),
        ),
    };
    Some(rule(
        "gpu-interface",
        "Graphics interface",
        "gpu",
        status,
        vec![part_ref(gpu), part_ref(board)],
        evidence,
        explanation,
        follow_up,
        "Compares parsed PCIe generations; a newer card in an older slot is a conflict, a slower card in a newer slot is not.",
    ))
}

fn gpu_length(parts: &BuildParts) -> Option<RuleResult> {
    let (gpu, case_part) = (parts.gpu.as_ref()?, parts.case_part.as_ref()?);
    let evidence = vec![
        field_evidence(gpu, "length"),
        field_evidence(case_part, "max_gpu_length"),
    ];
    let (length, clearance) = (
        number(&gpu.record, "length"),
        number(&case_part.record, "max_gpu_length"),
    );
    let (status, explanation, follow_up) = match (length, clearance) {
        (Some(length), Some(clearance)) if length <= clearance => (
            Verdict::Compatible,
            format!("Graphics card length {length} mm fits the recorded {clearance} mm case clearance."),
            None,
        ),
        (Some(length), Some(clearance)) => (
            Verdict::Incompatible,
            format!("Graphics card length {length} mm exceeds the recorded {clearance} mm case clearance."),
            None,
        ),
        _ => (
            Verdict::Unknown,
            "The graphics card length or case clearance is missing.".to_string(),
            Some(unknown_follow_up(
                &evidence,
                "Confirm the graphics card length and the case clearance available to it.",
            )),
        ),
    };
    Some(rule(
        "gpu-length",
        "Graphics card clearance",
        "gpu",
        status,
        vec![part_ref(gpu), part_ref(case_part)],
        evidence,
        explanation,
        follow_up,
        "Compares millimetre values only when both are recorded; shared or conditional bay space stays unknown.",
    ))
}

fn cooler_socket(parts: &BuildParts) -> Option<RuleResult> {
    let (cooler, board) = (parts.cooler.as_ref()?, parts.motherboard.as_ref()?);
    let evidence = vec![
        field_evidence(cooler, "cooler_socket"),
        field_evidence(board, "socket"),
    ];
    let (cooler_socket, board_socket) = (
        text(&cooler.record, "cooler_socket"),
        text(&board.record, "socket"),
    );
    let (status, explanation) = match (cooler_socket, board_socket) {
        (Some(a), Some(b)) if a.eq_ignore_ascii_case(b) => (
            Verdict::Compatible,
            format!("The cooler mounts on Socket {a}, matching the motherboard."),
        ),
        (Some(a), Some(b)) => (
            Verdict::Incompatible,
            format!("The cooler mounts on Socket {a} but the motherboard is Socket {b}."),
        ),
        _ => (
            Verdict::Unknown,
            "The cooler mounting Socket or motherboard Socket is missing.".to_string(),
        ),
    };
    Some(rule(
        "cooler-socket",
        "Cooler mounting",
        "cooling",
        status,
        vec![part_ref(cooler), part_ref(board)],
        evidence.clone(),
        explanation,
        (status == Verdict::Unknown).then(|| {
            unknown_follow_up(
                &evidence,
                "Confirm the cooler mounting kit and the motherboard Socket.",
            )
        }),
        "Exact mounting-Socket comparison; bracket compatibility is not inferred.",
    ))
}

fn cooler_clearance(parts: &BuildParts) -> Option<RuleResult> {
    let (cooler, case_part) = (parts.cooler.as_ref()?, parts.case_part.as_ref()?);
    let cooler_type = text(&cooler.record, "type")
        .unwrap_or_default()
        .to_lowercase();
    let liquid = cooler_type.contains("liquid")
        || cooler_type.contains("aio")
        || spec_of(&cooler.record, "radiator_size").is_some();
    if liquid {
        let evidence = vec![
            field_evidence(cooler, "radiator_size"),
            field_evidence(case_part, "front_radiator_mm"),
        ];
        let (size, clearance) = (
            number(&cooler.record, "radiator_size"),
            number(&case_part.record, "front_radiator_mm"),
        );
        let (status, explanation, follow_up) = match (size, clearance) {
            (Some(size), Some(clearance)) if size <= clearance => (
                Verdict::Compatible,
                format!("The {size} mm radiator fits the recorded {clearance} mm front clearance."),
                None,
            ),
            (Some(size), Some(clearance)) => (
                Verdict::Incompatible,
                format!("The {size} mm radiator exceeds the recorded {clearance} mm front clearance."),
                None,
            ),
            _ => (
                Verdict::Unknown,
                "Liquid-cooler radiator clearance depends on a case fact that is not recorded."
                    .to_string(),
                Some(unknown_follow_up(
                    &evidence,
                    "Confirm the case's front radiator clearance and the liquid cooler's radiator size.",
                )),
            ),
        };
        return Some(rule(
            "cooler-clearance",
            "Cooler clearance",
            "cooling",
            status,
            vec![part_ref(cooler), part_ref(case_part)],
            evidence,
            explanation,
            follow_up,
            "Liquid cooling compares radiator size with the recorded front clearance; conditional bay space is never assumed.",
        ));
    }
    let evidence = vec![
        field_evidence(cooler, "height"),
        field_evidence(case_part, "max_cooler_height"),
    ];
    let (height, clearance) = (
        number(&cooler.record, "height"),
        number(&case_part.record, "max_cooler_height"),
    );
    let (status, explanation, follow_up) = match (height, clearance) {
        (Some(height), Some(clearance)) if height <= clearance => (
            Verdict::Compatible,
            format!("Cooler height {height} mm fits the recorded {clearance} mm case clearance."),
            None,
        ),
        (Some(height), Some(clearance)) => (
            Verdict::Incompatible,
            format!(
                "Cooler height {height} mm exceeds the recorded {clearance} mm case clearance."
            ),
            None,
        ),
        _ => (
            Verdict::Unknown,
            "The cooler height or case cooler clearance is missing.".to_string(),
            Some(unknown_follow_up(
                &evidence,
                "Confirm the cooler height and the case CPU-cooler clearance.",
            )),
        ),
    };
    Some(rule(
        "cooler-clearance",
        "Cooler clearance",
        "cooling",
        status,
        vec![part_ref(cooler), part_ref(case_part)],
        evidence,
        explanation,
        follow_up,
        "Air cooling compares millimetre heights only when both facts are recorded.",
    ))
}

fn form_factor_rank(value: &str) -> Option<u8> {
    let normalized = value.trim().to_uppercase().replace([' ', '-', '_'], "");
    match normalized.as_str() {
        "EATX" | "EXTENDEDATX" => Some(4),
        "ATX" => Some(3),
        "MATX" | "MICROATX" => Some(2),
        "ITX" | "MINIITX" => Some(1),
        _ => None,
    }
}

fn board_case_form_factor(parts: &BuildParts) -> Option<RuleResult> {
    let (board, case_part) = (parts.motherboard.as_ref()?, parts.case_part.as_ref()?);
    let evidence = vec![
        field_evidence(board, "form_factor"),
        field_evidence(case_part, "supported_form_factors"),
    ];
    let board_value = text(&board.record, "form_factor");
    let case_value = text(&case_part.record, "supported_form_factors");
    let board_rank = board_value.and_then(form_factor_rank);
    let case_rank = case_value.and_then(|value| {
        value
            .split([',', '+', '/', '&'])
            .filter_map(form_factor_rank)
            .max()
    });
    let (status, explanation, follow_up) = match (board_rank, case_rank) {
        (Some(board_rank), Some(case_rank)) if board_rank <= case_rank => (
            Verdict::Compatible,
            format!(
                "The {} motherboard fits the case's declared {} support.",
                board_value.unwrap_or_default(),
                case_value.unwrap_or_default()
            ),
            None,
        ),
        (Some(_), Some(_)) => (
            Verdict::Incompatible,
            format!(
                "The motherboard is {} but the case supports at most {}.",
                board_value.unwrap_or_default(),
                case_value.unwrap_or_default()
            ),
            None,
        ),
        _ => (
            Verdict::Unknown,
            "The motherboard form factor or case support is missing or not a recognized form factor."
                .to_string(),
            Some(unknown_follow_up(
                &evidence,
                "Confirm the motherboard form factor and the largest form factor the case supports.",
            )),
        ),
    };
    Some(rule(
        "board-case-form-factor",
        "Motherboard and case form factor",
        "case",
        status,
        vec![part_ref(board), part_ref(case_part)],
        evidence,
        explanation,
        follow_up,
        "Compares a fixed E-ATX/ATX/micro-ATX/mini-ITX size order; unknown labels are not guessed.",
    ))
}

fn nominal_power(record: &Record) -> Option<f64> {
    number(record, "tdp").or_else(|| number(record, "power_draw"))
}

fn peak_power(record: &Record) -> Option<f64> {
    number(record, "peak_power")
}

fn psu_capacity(parts: &BuildParts) -> Option<RuleResult> {
    let (psu, cpu) = (parts.psu.as_ref()?, parts.cpu.as_ref()?);
    let mut evidence = vec![field_evidence(psu, "wattage"), field_evidence(cpu, "tdp")];
    let wattage = number(&psu.record, "wattage");
    let cpu_nominal = nominal_power(&cpu.record);
    let gpu_power = match parts.gpu.as_ref() {
        Some(gpu) => {
            evidence.push(field_evidence(gpu, "tdp"));
            evidence.push(field_evidence(gpu, "peak_power"));
            Some((nominal_power(&gpu.record), peak_power(&gpu.record)))
        }
        None => match is_yes(&cpu.record, "igpu") {
            Some(true) => Some((Some(0.0), Some(0.0))),
            _ => {
                evidence.push(model_evidence(cpu));
                None
            }
        },
    };
    let explanation = match (cpu_nominal, gpu_power) {
        (Some(_), Some(_)) => None,
        (None, _) => Some("The CPU nominal power draw is missing."),
        (_, None) => Some(
            "No graphics card is selected and the CPU has no sourced integrated-graphics fact.",
        ),
    };
    if let Some(explanation) = explanation {
        return Some(rule(
            "psu-capacity",
            "Power capacity screening",
            "power",
            Verdict::Unknown,
            vec![part_ref(psu), part_ref(cpu)],
            evidence.clone(),
            explanation.to_string(),
            Some(unknown_follow_up(
                &evidence,
                "Confirm the CPU and graphics power draw, or add a graphics card, before screening power.",
            )),
            "Nominal and sourced peak draw with fixed headroom; screening only, never a measurement.",
        ));
    }
    let cpu_nominal = cpu_nominal.unwrap_or(0.0);
    let (gpu_nominal, gpu_peak) = gpu_power.unwrap_or((Some(0.0), Some(0.0)));
    let nominal = cpu_nominal + gpu_nominal.unwrap_or(0.0);
    let (status, explanation, follow_up) = match wattage {
        None => (
            Verdict::Unknown,
            "The power supply wattage is missing.".to_string(),
            Some(unknown_follow_up(
                &evidence,
                "Confirm the power supply's rated wattage.",
            )),
        ),
        Some(wattage) if wattage < nominal => (
            Verdict::Incompatible,
            format!(
                "The power supply is rated {wattage} W, below the combined nominal CPU and graphics draw of {nominal} W."
            ),
            None,
        ),
        Some(wattage) => {
            let peak = match (peak_power(&cpu.record), gpu_peak) {
                (Some(cpu_peak), Some(gpu_peak)) => Some(cpu_peak + gpu_peak),
                _ => None,
            };
            match peak {
                Some(peak) if wattage >= peak + PSU_HEADROOM_W => (
                    Verdict::Compatible,
                    format!(
                        "The {wattage} W supply covers a sourced {peak} W peak plus {PSU_HEADROOM_W} W screening headroom."
                    ),
                    None,
                ),
                Some(peak) => (
                    Verdict::Unknown,
                    format!(
                        "The {wattage} W supply covers nominal draw but not the sourced {peak} W peak plus {PSU_HEADROOM_W} W screening headroom."
                    ),
                    Some(
                        "Confirm transient power behaviour or choose a supply with more headroom; this estimate is screening, not proof."
                            .to_string(),
                    ),
                ),
                None => (
                    Verdict::Unknown,
                    "Nominal draw fits, but peak or transient power is not sourced.".to_string(),
                    Some(
                        "Confirm sourced peak or transient power draw for the CPU and graphics card; nominal screening alone cannot prove headroom."
                            .to_string(),
                    ),
                ),
            }
        }
    };
    Some(rule(
        "psu-capacity",
        "Power capacity screening",
        "power",
        status,
        vec![part_ref(psu), part_ref(cpu)],
        evidence,
        explanation,
        follow_up,
        "Nominal draw plus sourced peak and a fixed 50 W screening headroom; only nominal insufficiency is a definite conflict.",
    ))
}

fn connector_counts(value: &str) -> Option<BTreeMap<u32, u32>> {
    let mut counts = BTreeMap::new();
    let mut found = false;
    for token in value.split([',', '+', '/', ' ']) {
        let token = token.trim().to_lowercase();
        let Some((count, pins)) = token.split_once('x') else {
            continue;
        };
        let Some(pins) = pins.strip_suffix("pin") else {
            continue;
        };
        let (Ok(count), Ok(pins)) = (count.parse::<u32>(), pins.parse::<u32>()) else {
            continue;
        };
        *counts.entry(pins).or_insert(0) += count;
        found = true;
    }
    found.then_some(counts)
}

fn psu_gpu_connectors(parts: &BuildParts) -> Option<RuleResult> {
    let (gpu, psu) = (parts.gpu.as_ref()?, parts.psu.as_ref()?);
    let evidence = vec![
        field_evidence(gpu, "connector"),
        field_evidence(psu, "psu_connectors"),
    ];
    let required = text(&gpu.record, "connector").and_then(connector_counts);
    let available = text(&psu.record, "psu_connectors").and_then(connector_counts);
    let (status, explanation, follow_up) = match (required, available) {
        (Some(required), Some(available)) => {
            let mut conflicts = Vec::new();
            for (pins, count) in &required {
                let have = available.get(pins).copied().unwrap_or(0);
                if have < *count {
                    conflicts.push(format!(
                        "the graphics card needs {count}×{pins}-pin but the supply lists {have}×{pins}-pin"
                    ));
                }
            }
            if conflicts.is_empty() {
                (
                    Verdict::Compatible,
                    "The supply lists every graphics-card power connector the card requires.".to_string(),
                    None,
                )
            } else {
                (
                    Verdict::Incompatible,
                    format!("Missing power connectors: {}.", conflicts.join("; ")),
                    None,
                )
            }
        }
        _ => (
            Verdict::Unknown,
            "The graphics-card connector requirement or supply connector list is missing.".to_string(),
            Some(unknown_follow_up(
                &evidence,
                "Confirm the graphics-card power connectors and the power supply's available connectors.",
            )),
        ),
    };
    Some(rule(
        "psu-gpu-connectors",
        "Graphics power connectors",
        "power",
        status,
        vec![part_ref(gpu), part_ref(psu)],
        evidence,
        explanation,
        follow_up,
        "Counts connector tokens by pin size; absent connector facts stay unknown.",
    ))
}

fn psu_case_mount(parts: &BuildParts) -> Option<RuleResult> {
    let (psu, case_part) = (parts.psu.as_ref()?, parts.case_part.as_ref()?);
    let evidence = vec![
        field_evidence(psu, "form_factor"),
        field_evidence(case_part, "psu_form_factor"),
        field_evidence(psu, "length"),
        field_evidence(case_part, "max_psu_length"),
    ];
    let form = (
        text(&psu.record, "form_factor"),
        text(&case_part.record, "psu_form_factor"),
    );
    let length = (
        number(&psu.record, "length"),
        number(&case_part.record, "max_psu_length"),
    );
    let (status, explanation, follow_up) = match (form, length) {
        ((Some(psu_form), Some(case_form)), (Some(psu_length), Some(max_length))) => {
            if !psu_form.eq_ignore_ascii_case(case_form) {
                (
                    Verdict::Incompatible,
                    format!("The supply is {psu_form} but the case accepts {case_form}."),
                    None,
                )
            } else if psu_length > max_length {
                (
                    Verdict::Incompatible,
                    format!(
                        "The supply is {psu_length} mm long, above the case's {max_length} mm limit."
                    ),
                    None,
                )
            } else {
                (
                    Verdict::Compatible,
                    format!(
                        "The {psu_form} supply at {psu_length} mm fits the case's {case_form} bay and {max_length} mm limit."
                    ),
                    None,
                )
            }
        }
        _ => (
            Verdict::Unknown,
            "The power-supply form factor, length, or case mounting limit is missing.".to_string(),
            Some(unknown_follow_up(
                &evidence,
                "Confirm the power-supply form factor and length and the case's supported supply bay.",
            )),
        ),
    };
    Some(rule(
        "psu-case-mount",
        "Power supply mounting",
        "power",
        status,
        vec![part_ref(psu), part_ref(case_part)],
        evidence,
        explanation,
        follow_up,
        "Compares the declared form factor and millimetre length; no bay geometry is inferred.",
    ))
}

fn normalize_interface(value: &str) -> String {
    value.trim().to_uppercase().replace([' ', '-', '_'], "")
}

fn storage_interface(parts: &BuildParts) -> Option<RuleResult> {
    let board = parts.motherboard.as_ref()?;
    if parts.storage.is_empty() {
        return None;
    }
    let mut evidence = vec![field_evidence(board, "storage_interface")];
    let mut status = Verdict::Compatible;
    let mut conflicts = Vec::new();
    for storage in &parts.storage {
        evidence.push(field_evidence(storage, "interface"));
        match (
            text(&board.record, "storage_interface"),
            text(&storage.record, "interface"),
        ) {
            (Some(board_interface), Some(interface)) => {
                if normalize_interface(board_interface) == normalize_interface(interface) {
                    // compatible
                } else {
                    status = status.merge(Verdict::Incompatible);
                    conflicts.push(format!(
                        "{} uses {interface} but the board lists {board_interface}.",
                        storage.record.sku
                    ));
                }
            }
            _ => status = status.merge(Verdict::Unknown),
        }
    }
    let explanation = if conflicts.is_empty() {
        "Every selected drive matches the recorded board storage interface.".to_string()
    } else {
        conflicts.join(" ")
    };
    Some(rule(
        "storage-interface",
        "Storage interface",
        "storage",
        status,
        std::iter::once(part_ref(board))
            .chain(parts.storage.iter().map(part_ref))
            .collect(),
        evidence.clone(),
        explanation,
        (status == Verdict::Unknown).then(|| {
            unknown_follow_up(
                &evidence,
                "Confirm the board storage interface and each drive interface.",
            )
        }),
        "Compares normalized interface names; a missing interface is unknown, not SATA by default.",
    ))
}

pub(super) fn evaluate_rules(parts: &BuildParts) -> Vec<RuleResult> {
    let mut rules = Vec::new();
    for candidate in [
        cpu_socket(parts),
        cpu_bios(parts),
        ram_type(parts),
        ram_capacity(parts),
        ram_slots(parts),
        gpu_interface(parts),
        gpu_length(parts),
        cooler_socket(parts),
        cooler_clearance(parts),
        board_case_form_factor(parts),
        psu_capacity(parts),
        psu_gpu_connectors(parts),
        psu_case_mount(parts),
        storage_interface(parts),
    ]
    .into_iter()
    .flatten()
    {
        rules.push(candidate);
    }
    rules.sort_by(|a, b| {
        category_rank(&a.category)
            .cmp(&category_rank(&b.category))
            .then_with(|| a.rule_id.cmp(&b.rule_id))
    });
    rules
}

#[cfg(test)]
mod tests {
    use super::pcie_generation;

    #[test]
    fn pcie_generation_reads_the_first_number() {
        assert_eq!(pcie_generation("PCIe 4.0 x16"), Some(4));
        assert_eq!(pcie_generation("PCIe 5.0"), Some(5));
        assert_eq!(pcie_generation("PCIe5"), Some(5));
        assert_eq!(pcie_generation("5.0"), Some(5));
        assert_eq!(pcie_generation("unknown"), None);
    }
}
