# Design: A sourced seed and catalog provenance

## Data model

`RowValues` gains two optional strings: `source` and `sourceUrl`. They are read from
two optional CSV columns and are never required.

`source` is a closed shape:

- empty — no catalog claim; behaviour is byte-identical to today;
- `user` — the operator asserts the facts; same as empty;
- `catalog:<id>` — the facts were copied from a named dataset revision, where `<id>`
  matches `[A-Za-z0-9._@/+-]+` (for example `catalog:buildcores@a1b2c3d` or
  `catalog:manufacturer@2026-10-31`).

Anything else is a row-level field error `source_format`. `sourceUrl` must be an
absolute `http://` or `https://` URL of at most 500 characters; anything else is
`source_url_format`. Invalid values are reported like every other field error and the
row stays excluded by default. They are never dropped silently.

## Provenance policy

| Truth | Fields | Provenance source | Reference |
| --- | --- | --- | --- |
| Catalog fact | `model`, each parsed spec | the declared `catalog:` value | `sourceUrl` when present, else `csv:<filename>` |
| Operator fact | `sku`, `quantity`, `price`, `currency` | `user` | `csv:<filename>` |
| Derived | `category`, `modelNormalized` | `rigcraft-normalizer` | the normalizer version |

A catalog claim never overwrites an operator value. If the operator corrects `model` or
`specs` for a row, `apply_overrides` clears that row's `source` and `sourceUrl` before
re-evaluation, so the edited facts cite `user` with the correction reference. The
correction endpoint already writes `user` provenance for every changed field.

`MatchInfo` becomes `catalog-declared` with a basis naming the dataset when a catalog
source is present, and stays `unverified` otherwise. This says exactly what happened:
RigCraft recorded a declared source, it did not query the dataset or verify the claim.

## Why the catalog claim is limited to model and specs

The CSV's SKU is the operator's stock key, not necessarily the manufacturer part number,
and price and quantity are inventory facts that no catalog can supply (OpenDB carries no
prices). Claiming a catalog source for them would be false. Category is a lexical mapping
performed by RigCraft, so the normalizer owns it.

## Shipped seed

Ten rows, one physically possible AM5 machine, eight categories, facts checked on
2026-10-31 against the manufacturer's own page:

| Row | Part | Manufacturer page |
| --- | --- | --- |
| CPU-7800X3D | AMD Ryzen 7 7800X3D | `https://www.amd.com/en/products/processors/desktops/ryzen/7000-series/amd-ryzen-7-7800x3d.html` |
| MB-B650E-F | ASUS ROG STRIX B650E-F GAMING WIFI | `https://rog.asus.com/motherboards/rog-strix/rog-strix-b650e-f-gaming-wifi-model/spec/` |
| RAM-FURY-32-6000 | Kingston FURY Beast 32GB (2×16GB) DDR5-6000 CL36 | `https://www.kingston.com/dataSheets/KF560C36BBEK2-32.pdf` |
| GPU-4070-FE | NVIDIA GeForce RTX 4070 Founders Edition | `https://www.nvidia.com/en-us/geforce/graphics-cards/40-series/rtx-4070-4070ti/` |
| PSU-RM750E | Corsair RM750e | `https://www.corsair.com/us/en/p/psu/cp-9020262-na/rm750e-fully-modular-low-noise-atx-power-supply-cp-9020262-na` |
| CASE-4000D | Corsair 4000D AIRFLOW | `https://www.corsair.com/us/en/p/case/cc-9011200-ww/4000d-airflow-tempered-glass-mid-tower-atx-case-cc-9011200-ww` |
| COOL-PA120SE | Thermalright Peerless Assassin 120 SE | `https://www.thermalright.com/product/peerless-assassin-120-se/` |
| SSD-MP600-1TB | Corsair MP600 PRO NH 1TB | `https://www.corsair.com/us/en/p/data-storage/cssd-f1000gbmp600pnh/mp600-pro-nh-1tb-pcie-gen4-x4-nvme-m-2-ssd-cssd-f1000gbmp600pnh` |
| SSD-MP600-2TB | Corsair MP600 PRO NH 2TB | `https://www.corsair.com/us/en/p/data-storage/cssd-f2000gbmp600pnh/mp600-pro-nh-2tb-pcie-gen4-x4-nvme-m-2-ssd-cssd-f2000gbmp600pnh` |
| SSD-MP600-4TB | Corsair MP600 PRO NH 4TB | `https://www.corsair.com/us/en/p/data-storage/cssd-f4000gbmp600pnh/mp600-pro-nh-4tb-pcie-gen4-x4-nvme-m-2-ssd-cssd-f4000gbmp600pnh` |

The Samsung 990 PRO candidate was dropped: its product and data-sheet pages are
JS-rendered or redirect, so no manufacturer page could be cited. A Corsair MP600 PRO NH
was substituted from a page that states the interface, form factor and capacity in
server-rendered text.

Quantity and unit price are the sample's operator-entered stock values, not market data
and not catalog facts. The docs and the download area say so.

The shipped rows omit `bios_supports` and any peak/transient power figure because no
cited page publishes them. Consequently `cpu-bios` and `psu-capacity` report `unknown`
with a follow-up; they do not report a conflict. That is the intended demonstration of
unknown-never-guessed.

## OpenDB converter

`scripts/opendb-to-csv.mjs <checkout> --price <decimal> --quantity <integer>` reads the
eight supported folders, derives the commit SHA with `git -C <checkout> rev-parse HEAD`
(override with `--commit`), and writes the import CSV to stdout or `--out`.

Mapping, and nothing beyond it:

| OpenDB folder | category | model | SKU | specs |
| --- | --- | --- | --- | --- |
| `CPU` | cpu | `metadata.name` | mpn, else part number, else `opendb_id` | socket, specifications.tdp W, igpu=yes only when an integrated model is recorded |
| `Motherboard` | motherboard | `metadata.name` | same | socket, form_factor, memory.ram_type/slots/max, highest PCIe generation, M.2 interface only when every M.2 slot records the same interface |
| `RAM` | ram | `metadata.name` | same | ram_type, capacity GB, modules.quantity |
| `GPU` | gpu | `metadata.name` | same | interface, length mm, tdp W, memory GB, connector tokens from `power_connectors` |
| `PSU` | psu | `metadata.name` | same | wattage W, form_factor, length mm, 8-pin and 16-pin counts from `connectors` |
| `PCCase` | case | `metadata.name` | same | supported_motherboard_form_factors, max_video_card_length, max_cpu_cooler_height, max_psu_length, psu form factor only when ATX is supported |
| `CPUCooler` | cooler | `metadata.name` | same | height mm, cooler_socket only when exactly one socket is listed |
| `Storage` | storage | `metadata.name` | same | capacity GB, `M.2 ` prefix stripped from interface, form_factor |

Every mapped value is emitted only when OpenDB records it; a null is omitted so the
fact stays unknown. A singular field that OpenDB records as a set (a cooler's sockets,
a board's mixed M.2 interfaces) is omitted rather than reduced to a guess.

The converter exits non-zero, naming the file and field, when a requested category is
unmapped, a part has no name, a part has no usable SKU, the JSON cannot be parsed, or
`--price` / `--quantity` is missing. It never invents a price from catalog data because
OpenDB contains none.

Two real-data conditions are warnings rather than failures, because both have an honest
consumer-side resolution: a shared MPN is emitted twice so the import's duplicate
decision handles it, and an unusable manufacturer URL is omitted so the row keeps a valid
provenance reference (the CSV filename). A model name beyond the import's field limit is
emitted unchanged with a warning; the preview reports it and the operator decides. An MPN
beyond the 120-character SKU limit falls back to the record's `opendb_id` with a warning.
On a 2026-10-31 sparse checkout (commit
`d8d0149dbb32f75e9408d0476b231984c1fc2619`) the four tested categories produced 10,527
rows with 607 warnings, of which 9,619 were valid in the preview, 2 were flagged
`too_long` for their model name, and 906 rows formed duplicate groups.

## Licence architecture

The repository's code stays MIT. The compiled sample rows are licensed CC BY 4.0 by
RigCraft contributors with attribution in `NOTICE`; the underlying product names and
marks remain their owners'. OpenDB is never committed: the operator fetches it, and its
CC BY-NC-SA 4.0 obligations (attribution, non-commercial use, ShareAlike) apply to that
operator. `docs/data-sources.md` records the assessment, including sources rejected on
licence grounds.

## UI

- `SpecBadge` derives its label from the spec's provenance: `Operator-confirmed` (✎),
  `Catalog` (▣) or `Unknown` (?). Not colour alone.
- Each record and preview row shows `Catalog facts · <id>` or `Operator-entered facts`
  and keeps the existing match-confidence wording.
- The import form's download area states the sample's CC BY 4.0 licence, the
  2026-10-31 verification date and that prices are operator-entered.
- `sourceLabel` maps `catalog:<id>` to `Catalog · <id>` and leaves existing labels.
- One new measured contrast pair, `Catalog badge on muted` (`color-primary` on
  `color-muted`), joins `scripts/check-contrast.mjs` and `docs/evidence/token-contrast.json`.

## Failure and compatibility

Existing CSVs without the columns keep their exact behaviour and provenance. A CSV that
already has a `Source` column now maps it to the new logical field; this is documented in
`docs/inventory-import.md` and is the only re-interpretation. The SQLite schema does not
change; provenance is stored in the existing record JSON. No new dependency is added:
the converter uses only `node:fs`, `node:path`, `node:child_process` and `node:process`.
