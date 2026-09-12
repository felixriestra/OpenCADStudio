<p align="center">
  <a href="../../README.md">English</a> · <a href="README.bg.md">Български</a> · <a href="README.pt-BR.md">Português (Brasil)</a> · <a href="README.cs.md">Čeština</a> · <a href="README.nl.md">Nederlands</a> · <a href="README.fr.md">Français</a> · <a href="README.fi.md">Suomi</a> · <a href="README.de.md">Deutsch</a> · <a href="README.el.md">Ελληνικά</a> · <a href="README.hu.md">Magyar</a> · <a href="README.it.md">Italiano</a> · <a href="README.ja.md">日本語</a> · <a href="README.ko.md">한국어</a> · <a href="README.pl.md">Polski</a> · <a href="README.ru.md">Русский</a> · <a href="README.zh-CN.md">简体中文</a> · <a href="README.es.md">Español</a> · <a href="README.zh-TW.md">繁體中文</a> · <a href="README.tr.md">Türkçe</a> · <a href="README.hi.md">हिन्दी</a> · <a href="README.ar.md">العربية</a>
</p>

<p align="center"><img src="../../assets/logo.svg" width="112" alt="Logo Mac2CAM"></p>
<h1 align="center">Mac2CAM</h1>
<p align="center">Otwarte narzędzie do rysowania 2D i modelowania 3D na komputerze i w przeglądarce, napisane w Rust.</p>

<p align="center">
  <a href="https://github.com/HakanSeven12/Mac2CAM/releases/latest"><img alt="Najnowsze wydanie" src="https://img.shields.io/github/v/release/HakanSeven12/Mac2CAM"></a>
  <a href="https://github.com/HakanSeven12/Mac2CAM/releases"><img alt="Pobrania" src="https://img.shields.io/github/downloads/HakanSeven12/Mac2CAM/total"></a>
  <a href="https://github.com/HakanSeven12/Mac2CAM/stargazers"><img alt="Gwiazdki GitHub" src="https://img.shields.io/github/stars/HakanSeven12/Mac2CAM"></a>
  <a href="../../LICENSE"><img alt="Licencja GPL-3.0" src="https://img.shields.io/github/license/HakanSeven12/Mac2CAM"></a>
</p>

<p align="center">
  <a href="https://www.mac2cam.com"><strong>Uruchom aplikację webową</strong></a> ·
  <a href="https://github.com/HakanSeven12/Mac2CAM/releases/latest"><strong>Pobierz aplikację komputerową</strong></a> ·
  <a href="https://github.com/HakanSeven12/Mac2CAM/discussions"><strong>Dołącz do dyskusji</strong></a>
</p>

<p align="center"><img src="../../site/workspace.png" alt="Obszar roboczy Mac2CAM" width="100%"></p>

## Przegląd

Mac2CAM to wieloplatformowa aplikacja do rysunku technicznego, pracy z układami i modelowania brył. Natywnie odczytuje i zapisuje rysunki DWG i DXF, a wersje komputerowa i przeglądarkowa korzystają ze wspólnego rdzenia edycji.

Projekt jest aktywnie rozwijany. Zachowuj kopie zapasowe ważnych rysunków produkcyjnych, a powtarzalne problemy zgłaszaj przez [GitHub Issues](https://github.com/HakanSeven12/Mac2CAM/issues).

## Najważniejsze funkcje

- **Natywna praca z rysunkiem** — otwieranie, edycja, odzyskiwanie i zapis plików DWG oraz DXF bez usługi konwersji.
- **Precyzyjne rysowanie 2D** — linie, polilinie, krzywe, splajny, kreskowania, przyciąganie do obiektów, śledzenie, warstwy, bloki i odnośniki zewnętrzne.
- **Narzędzia dokumentacji** — tekst, wymiary, odnośniki, tolerancje, tabele, obszar modelu, obszar papieru, rzutnie i style wydruku.
- **Modelowanie 3D oparte na jądrze** — prymitywy bryłowe, wyciąganie, obrót, przeciągnięcie, loft, operacje logiczne i teselacja elementów ACIS.
- **Renderowanie GPU** — przyspieszone widoki 2D i 3D przez `wgpu`, z kamerą ortograficzną i perspektywiczną.
- **Rozszerzalne przepływy pracy** — natywne wtyczki, skrypty poleceń, konwersja bez interfejsu i wierszowe API automatyzacji JSON.

<p align="center"><img src="../../site/modeling.png" alt="Model 3D w Mac2CAM" width="100%"></p>

## Przepływy plików

| Format lub przepływ | Obsługa |
| --- | --- |
| DWG | Odczyt i zapis; wersjonowane cele zapisu od R14 do 2018 |
| DXF | Odczyt i zapis; wersjonowane cele zapisu od R14 do 2018 |
| BAK / SV$ | Otwieranie kopii zapasowych i plików autozapisu |
| OBJ | Import siatek wielokątów |
| LandXML | Import punktów pomiarowych `CgPoint` |
| STL | Eksport danych siatki 3D |
| STEP AP203 | Eksport danych siatki 3D |
| PDF | Drukowanie układów i wybranej geometrii w wersji komputerowej |
| CSV | Wyodrębnianie danych właściwości elementów |
| CTB / STB | Wczytywanie i edycja tabel stylów wydruku |

## Komputer czy przeglądarka

Użyj [aplikacji webowej](https://www.mac2cam.com), aby rozpocząć bez instalacji. Rysunki wybiera się w przeglądarce i zapisuje jako lokalne pliki do pobrania.

Użyj aplikacji komputerowej do natywnych skojarzeń plików, miniatur w menedżerze plików, drukowania systemowego, wyjścia PDF, zewnętrznych wtyczek, skryptów poleceń i automatyzacji bez interfejsu. Wydania są dostępne dla Windows, Linux i macOS na Apple Silicon.

## Instalacja

Wszystkie aktualne pakiety pobierzesz z [najnowszego wydania](https://github.com/HakanSeven12/Mac2CAM/releases/latest).

### Windows

Wybierz jeden z podpisanych pakietów x86-64:

- `Mac2CAM-*-windows-x86_64-installer.msi` — zalecany instalator ze skrótami menu Start, skojarzeniami DWG/DXF i miniaturami rysunków.
- `Mac2CAM-*-windows-x86_64-portable.exe` — samodzielna aplikacja niewymagająca instalacji.

### Linux

Pobierz x86-64 AppImage, nadaj mu uprawnienia wykonywania i uruchom:

```bash
chmod +x Mac2CAM-*-linux-x86_64.AppImage
./Mac2CAM-*-linux-x86_64.AppImage
```

### macOS

Opublikowany pakiet macOS obsługuje Apple Silicon:

1. Pobierz `Mac2CAM-*-macos-arm64.dmg`.
2. Otwórz obraz i przeciągnij `Mac2CAM.app` do **Applications**.
3. Jeśli Gatekeeper zablokuje pierwsze uruchomienie, zatwierdź aplikację w **System Settings → Privacy & Security**.

Aplikacja jest podpisana ad hoc, ale obecnie nie jest notaryzowana przez Apple.

## Języki

Mac2CAM może korzystać z języka systemu lub jednego z 21 języków interfejsu:

> Arabski · Portugalski brazylijski · Bułgarski · Czeski · Niderlandzki · Angielski · Fiński · Francuski · Niemiecki · Grecki · Hindi · Węgierski · Włoski · Japoński · Koreański · Polski · Rosyjski · Chiński uproszczony · Hiszpański · Chiński tradycyjny · Turecki

Język zmienisz w ustawieniach aplikacji. Gdy wybrano **System**, wersja przeglądarkowa używa także preferowanych ustawień regionalnych przeglądarki.

## Budowanie ze źródeł

### Wersja komputerowa

Wymagania:

- Git
- Aktualny stabilny zestaw narzędzi Rust
- Biblioteki programistyczne grafiki i czcionek dla platformy

W Ubuntu lub Debianie zainstaluj natywne zależności:

```bash
sudo apt update
sudo apt install libgl1-mesa-dev libx11-dev libxcursor-dev libxi-dev \
  libxrandr-dev libxkbcommon-dev libwayland-dev libfontconfig1-dev \
  libfreetype6-dev
```

Następnie zbuduj:

```bash
git clone https://github.com/HakanSeven12/Mac2CAM.git
cd Mac2CAM
cargo build --release --bin Mac2CAM
```

Powstały plik wykonywalny znajduje się w `target/release/Mac2CAM` (`Mac2CAM.exe` w Windows).

### Web

Jednorazowo zainstaluj cel WebAssembly i narzędzia budowania:

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk wasm-bindgen-cli
```

Uruchom serwer programistyczny:

```bash
trunk serve
```

## Automatyzacja

Program komputerowy obsługuje jednorazową konwersję i trwały serwer bez interfejsu:

```bash
Mac2CAM --export input.dwg output.dxf
Mac2CAM --serve
Mac2CAM --serve --port 4242
Mac2CAM --mcp
```

Serwer wymienia jeden obiekt JSON na wiersz przez standardowe wejście/wyjście lub lokalne gniazdo TCP. Zobacz [przewodnik automatyzacji](../automation/README.md).

## Wtyczki

Wtyczki komputerowe działają w oddzielnych procesach i komunikują się z hostem przez wersjonowane API wtyczek. Wersja przeglądarkowa nie ładuje natywnych wtyczek.

- [Architektura wtyczek](../plugin-architecture.md)
- [Szablon wtyczki](../plugin-template/README.md)
- [Rejestr wtyczek](../../plugins/README.md)

## Dokumentacja projektu

- [API automatyzacji](../automation/README.md)
- [Architektura wtyczek](../plugin-architecture.md)
- [Potok teselacji](../tessellation.md)
- [Zasady bezpieczeństwa](../../SECURITY.md)

## Współtworzenie

Mile widziane są zgłoszenia błędów, ukierunkowane pull requesty, tłumaczenia, ulepszenia dokumentacji i wkład we wtyczki.

- Przed otwarciem nowego zgłoszenia przeszukaj istniejące [issues](https://github.com/HakanSeven12/Mac2CAM/issues).
- Do pytań i pomysłów używaj [Discussions](https://github.com/HakanSeven12/Mac2CAM/discussions).
- Luki zgłaszaj prywatnie zgodnie z [zasadami bezpieczeństwa](../../SECURITY.md).

## Rozwój projektu

<a href="https://github.com/HakanSeven12/Mac2CAM/stargazers">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://www.mac2cam.com/star-history-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="https://www.mac2cam.com/star-history-light.svg">
    <img alt="Gwiazdki i pobrania wydań Mac2CAM" src="https://www.mac2cam.com/star-history-light.svg">
  </picture>
</a>

## Wesprzyj projekt

Jeśli Mac2CAM pomaga w Twojej pracy, wesprzyj dalszy rozwój przez [GitHub Sponsors](https://github.com/sponsors/HakanSeven12) lub [Patreon](https://www.patreon.com/HakanSeven12).

## Licencja

Mac2CAM jest rozpowszechniane na warunkach [GNU General Public License v3.0](../../LICENSE).
