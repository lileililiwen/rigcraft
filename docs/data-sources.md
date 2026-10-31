# Data sources and licensing position

Status: recorded by `real-catalog-seed-and-import` (2026-10-31).

RigCraft is a non-commercial open-source project. Its **code** is MIT (`LICENSE`). Its
**shipped sample rows** are CC BY 4.0 (`NOTICE`). It does **not** vendor a third-party
hardware dataset. This document records what was assessed, what was used, and why the
large catalog is fetched by the operator instead of committed.

## Assessment

| Source | Maintenance evidence | Licence | Commercial position | Field fit for `compat-rules-v1` | Verdict |
| --- | --- | --- | --- | --- | --- |
| [BuildCores OpenDB](https://github.com/buildcores/buildcores-open-db) | Bot-synced repository; `main` pushed 2026-10-31T06:30Z; per-category JSON validated by `schemas/*.schema.json` | CC BY-NC-SA 4.0 per its [NOTICE.md](https://github.com/buildcores/buildcores-open-db/blob/main/NOTICE.md); GitHub reports "Other" because the licence is not an SPDX-listed file | Non-commercial only; ShareAlike applies to adapted database rights; commercial use needs a separate grant | Strong: `PCCase.supported_motherboard_form_factors`, `max_video_card_length`, `max_cpu_cooler_height`, `max_psu_length`, `supported_power_supply_form_factors`; `CPU.socket` and `specifications.tdp`; `Motherboard.memory`, `pcie_slots`, `m2_slots`; `GPU.interface`, `length`, `tdp`, `power_connectors`; `PSU.wattage`, `form_factor`, `length`, `connectors`; `CPUCooler.height`, `cpu_sockets`; `Storage.interface`, `capacity`. No prices. | **Consume by operator fetch, never vendor.** Shipping it inside an MIT repository would misstate downstream rights and add roughly 118 MB. `scripts/opendb-to-csv.mjs` is the supported path. |
| [TechFuelHQ pc-builder-parts](https://github.com/iBlessi/pc-builder-parts) | Repository pushed 2026-10-01; README records v2.1.1 on 2026-09-29 with 126 parts, per-row `last_verified`, and per-row `source` on 97 of them | CC BY 4.0 (GitHub licence API: `CC-BY-4.0`) | Redistribution permitted with attribution | Moderate: CPU/GPU/motherboard/RAM/storage/PSU/case only, no cooler rows, no clearance values for coolers, no prices. | **Assessed, not vendored in this change.** It is redistributable, but manufacturer pages are the stronger primary source for a ten-row seed, and copying it would require its own attribution and a statement of changes for no extra capability. |
| [PCPartPicker](https://pcpartpicker.com/help/terms-of-service) | Live site with prices | Site terms | Terms prohibit automated collection and derivative use | Good product data, but unusable | **Never used.** No code, fixture, script or seed row is derived from it. |
| [Octopart / Nexar](https://www.octopart.com/) | Commercial aggregator API | Commercial terms, paid | Paid supply-chain access | Poor: distributor pricing and stock, not chassis or cooler clearance | Not a seed source. |
| [Open Icecat](https://icecat.biz/) | Brand datasheet syndication | Vendor terms: account and token at runtime; use of the content for machine-learning training is restricted | Non-commercial use still needs the vendor account | Partial: marketing datasheets, inconsistent clearance fields | Not a seed source. |

## Shipped sample

Ten rows, one build, every compatibility fact checked against the manufacturer's own
page on **2026-10-31**. `source` is `catalog:manufacturer@2026-10-31`; `sourceUrl` is the
page below.

| SKU | Part | Facts recorded | Page checked |
| --- | --- | --- | --- |
| `CPU-7800X3D` | AMD Ryzen 7 7800X3D | Socket AM5, default TDP 120 W, integrated Radeon graphics | <https://www.amd.com/en/products/processors/desktops/ryzen/7000-series/amd-ryzen-7-7800x3d.html> |
| `MB-B650E-F` | ASUS ROG STRIX B650E-F GAMING WIFI | Socket AM5, ATX, DDR5, 4 DIMM, max 192 GB, PCIe 5.0 x16, M.2_2/M.2_3 at PCIe 4.0 x4 (the seeded drive occupies M.2_2; M.2_1 is PCIe 5.0) | <https://rog.asus.com/motherboards/rog-strix/rog-strix-b650e-f-gaming-wifi-model/spec/> |
| `RAM-FURY-32-6000` | Kingston FURY Beast 32GB (2x16GB) DDR5-6000 CL36 | DDR5, 32 GB kit, 2 modules | <https://www.kingston.com/dataSheets/KF560C36BBEK2-32.pdf> |
| `GPU-4070-FE` | NVIDIA GeForce RTX 4070 Founders Edition | 244 mm, PCIe 4.0, 200 W total graphics power, 12 GB, 2x 8-pin or 16-pin cable | <https://www.nvidia.com/en-us/geforce/graphics-cards/40-series/rtx-4070-4070ti/> |
| `PSU-RM750E` | Corsair RM750e | 750 W, ATX 3.1, 140x150x86 mm, one 12V-2x6 and three PCIe 8-pin connectors | <https://www.corsair.com/us/en/p/psu/cp-9020262-na/rm750e-fully-modular-low-noise-atx-power-supply-cp-9020262-na> |
| `CASE-4000D` | Corsair 4000D AIRFLOW | E-ATX/ATX/micro-ATX/mini-ITX, 360 mm GPU, 170 mm cooler, 220 mm PSU, ATX supply | <https://www.corsair.com/us/en/p/case/cc-9011200-ww/4000d-airflow-tempered-glass-mid-tower-atx-case-cc-9011200-ww> |
| `COOL-PA120SE` | Thermalright Peerless Assassin 120 SE | 155 mm tall, AM4/AM5, air | <https://www.thermalright.com/product/peerless-assassin-120-se/> |
| `SSD-MP600-1TB` | Corsair MP600 PRO NH 1TB | PCIe 4.0 x4, M.2 2280, 1 TB | <https://www.corsair.com/us/en/p/data-storage/cssd-f1000gbmp600pnh/mp600-pro-nh-1tb-pcie-gen4-x4-nvme-m-2-ssd-cssd-f1000gbmp600pnh> |
| `SSD-MP600-2TB` | Corsair MP600 PRO NH 2TB | PCIe 4.0 x4, M.2 2280, 2 TB | <https://www.corsair.com/us/en/p/data-storage/cssd-f2000gbmp600pnh/mp600-pro-nh-2tb-pcie-gen4-x4-nvme-m-2-ssd-cssd-f2000gbmp600pnh> |
| `SSD-MP600-4TB` | Corsair MP600 PRO NH 4TB | PCIe 4.0 x4, M.2 2280, 4 TB | <https://www.corsair.com/us/en/p/data-storage/cssd-f4000gbmp600pnh/mp600-pro-nh-4tb-pcie-gen4-x4-nvme-m-2-ssd-cssd-f4000gbmp600pnh> |

Two facts are deliberately absent because the cited pages do not publish them: the
motherboard's per-CPU BIOS support statement and peak/transient power for the CPU or
graphics card. The compatibility check therefore reports `unknown` for `cpu-bios` and
`psu-capacity` and no conflict for the other twelve rules. That result is the point: a
missing fact stays missing.

The Samsung 990 PRO was considered for the storage rows but its product and data-sheet
pages either redirect to a generic listing or render their specification tables with
JavaScript, so no manufacturer page could be cited. The Corsair MP600 PRO NH rows use a
page that states interface, form factor and capacity in server-rendered text. Product
names and marks remain their owners'.

Quantity and unit price in the sample are **operator-entered stock values**. They are
not manufacturer facts, market data, or a price feed.

## Operator-supplied catalogs

RigCraft ships no catalog. An operator who is licensed to use one can upload it
through `/catalog` as a `rigcraft-catalog` version 1 document. The document, its
licence and its obligations remain the operator's: RigCraft stores it in that
instance's SQLite database, shows the licence verbatim, and uses it only for
search and for filling fields an import row leaves empty. It never commits the
catalog to the repository, writes it into a fixture or export template, or sends it
to another instance. The format, validation rules and matching order are in
[catalog-ingestion.md](catalog-ingestion.md).

The companion converter turns a local OpenDB checkout into that document:

```sh
node scripts/opendb-to-catalog.mjs /tmp/opencode/buildcores-open-db \
  --name buildcores-opendb --license "CC BY-NC-SA 4.0" \
  --out /tmp/opencode/catalog.json
```

It requires the dataset name and licence, resolves the checkout commit, refuses a
shared SKU rather than writing a document the server would reject, and adds no
price or quantity.

## Importing a local BuildCores OpenDB checkout

OpenDB is not distributed with RigCraft. Download it yourself and keep its licence
notice:

```sh
git clone https://github.com/buildcores/buildcores-open-db.git /tmp/opencode/buildcores-open-db
cd /tmp/opencode/buildcores-open-db && git rev-parse HEAD
```

Then convert the eight supported categories. `--price` and `--quantity` are required
because OpenDB carries neither; they fill the operator columns of the import CSV.

```sh
node scripts/opendb-to-csv.mjs /tmp/opencode/buildcores-open-db \
  --price 1.00 --quantity 1 --out /tmp/opencode/opendb.csv
```

The output header is `sku,category,model,quantity,price,specs,source,sourceUrl`.
`source` is `catalog:buildcores@<commit>` and `sourceUrl` is
`general_product_information.manufacturer_url` when OpenDB records one. Import it
through `/inventory`; the preview labels the model and specifications as catalog facts
and keeps SKU, quantity and price as operator facts.

Options:

- `--commit <sha>` — use an explicit revision instead of `git rev-parse HEAD`.
- `--only <Folder>` — convert one folder; repeat for several. The folders are `CPU`,
  `Motherboard`, `RAM`, `GPU`, `PSU`, `PCCase`, `CPUCooler`, `Storage`.
- `--out <file>` — write a file instead of stdout.

The converter exits non-zero and writes nothing when a requested category is unmapped,
a part has no name or usable SKU, a JSON file cannot be parsed, or
`--price`/`--quantity` is missing. A fact OpenDB does not record is omitted, never
guessed. A set-valued field that the engine models as one value (a cooler's supported
sockets, mixed board M.2 interfaces) is omitted unless it has a single unambiguous
value. A manufacturer URL that is not absolute HTTP(S) is omitted with a warning; a
shared MPN is emitted twice with a warning so the import preview's duplicate decision
resolves it; an MPN beyond the import's 120-character SKU limit is replaced by the
record's `opendb_id` with a warning, so the row stays importable; a model name beyond the
import's 200-character limit is emitted unchanged with a warning, and the preview reports
the row for the operator to correct or exclude.

Checked against a real sparse checkout (2026-10-31, commit
`d8d0149dbb32f75e9408d0476b231984c1fc2619`, `CPU`, `PCCase`, `CPUCooler` and
`Storage`): 10,527 rows converted with 607 warnings; the import preview accepted the
file with 9,619 valid rows, 2 rows flagged `model too_long`, and 906 duplicate rows.
Every row's `model` and `specs` cited `catalog:buildcores@<commit>`, while its SKU,
quantity and price cited `user`.

### Obligations for an operator who fetches OpenDB

OpenDB is CC BY-NC-SA 4.0. If you share the data or an adaptation, credit BuildCores
OpenDB, link the licence, say what you changed, and follow the ShareAlike and
database-rights terms. Non-commercial use only; commercial use needs a separate grant
from the rights holders. RigCraft's MIT licence does not extend to that data.

## What RigCraft itself never does

- It does not commit OpenDB or any other third-party dataset.
- It does not scrape PCPartPicker or use a PCPartPicker-derived dataset.
- It does not fetch a catalog at runtime; the operator runs the converter locally
  and uploads a catalog they are licensed to use.
- It does not redistribute, re-export or share an uploaded catalog with another
  instance.
- It does not fill a missing specification from a similar model name, and it never
  guesses a unit or overwrites an operator value with a catalog claim.
