<p align="center">
  <a href="../../README.md">English</a> · <a href="README.bg.md">Български</a> · <a href="README.pt-BR.md">Português (Brasil)</a> · <a href="README.cs.md">Čeština</a> · <a href="README.nl.md">Nederlands</a> · <a href="README.fr.md">Français</a> · <a href="README.fi.md">Suomi</a> · <a href="README.de.md">Deutsch</a> · <a href="README.el.md">Ελληνικά</a> · <a href="README.hu.md">Magyar</a> · <a href="README.it.md">Italiano</a> · <a href="README.ja.md">日本語</a> · <a href="README.ko.md">한국어</a> · <a href="README.pl.md">Polski</a> · <a href="README.ru.md">Русский</a> · <a href="README.zh-CN.md">简体中文</a> · <a href="README.es.md">Español</a> · <a href="README.zh-TW.md">繁體中文</a> · <a href="README.tr.md">Türkçe</a> · <a href="README.hi.md">हिन्दी</a> · <a href="README.ar.md">العربية</a>
</p>

<p align="center"><img src="../../assets/logo.svg" width="112" alt="Лого на Mac2CAM"></p>

<h1 align="center">Mac2CAM</h1>

<p align="center">Приложение с отворен код за 2D чертане и 3D моделиране за настолни системи и уеб, разработено с Rust.</p>

<p align="center">
  <a href="https://github.com/HakanSeven12/Mac2CAM/releases/latest"><img alt="Последна версия" src="https://img.shields.io/github/v/release/HakanSeven12/Mac2CAM"></a>
  <a href="https://github.com/HakanSeven12/Mac2CAM/releases"><img alt="Изтегляния на версии" src="https://img.shields.io/github/downloads/HakanSeven12/Mac2CAM/total"></a>
  <a href="https://github.com/HakanSeven12/Mac2CAM/stargazers"><img alt="Звезди в GitHub" src="https://img.shields.io/github/stars/HakanSeven12/Mac2CAM"></a>
  <a href="../../LICENSE"><img alt="Лиценз GPL-3.0" src="https://img.shields.io/github/license/HakanSeven12/Mac2CAM"></a>
</p>

<p align="center">
  <a href="https://www.mac2cam.com"><strong>Отворете уеб приложението</strong></a> ·
  <a href="https://github.com/HakanSeven12/Mac2CAM/releases/latest"><strong>Изтеглете настолното приложение</strong></a> ·
  <a href="https://github.com/HakanSeven12/Mac2CAM/discussions"><strong>Присъединете се към дискусията</strong></a>
</p>

<p align="center"><img src="../../site/workspace.png" alt="Работно пространство на Mac2CAM" width="100%"></p>

## Общ преглед

Mac2CAM е междуплатформено приложение за техническо чертане, работа с оформления и моделиране на твърди тела. То чете и записва DWG и DXF чертежи директно, като настолната и браузърната версия използват общо ядро за редактиране.

Проектът се разработва активно. Пазете резервни копия на важните производствени чертежи и съобщавайте възпроизводими проблеми чрез [GitHub Issues](https://github.com/HakanSeven12/Mac2CAM/issues).

## Основни възможности

- **Директен работен процес с чертежи** — отваряйте, редактирайте, възстановявайте и записвайте DWG и DXF файлове без услуга за преобразуване.
- **Прецизно 2D чертане** — линии, полилинии, криви, сплайни, щриховки, обектно прихващане, проследяване, слоеве, блокове и външни препратки.
- **Инструменти за документация** — текст, размери, указателни линии, допуски, таблици, моделно пространство, листово пространство, изгледи и стилове за плотиране.
- **3D моделиране с геометрично ядро** — твърдотелни примитиви, екструдиране, завъртане, изтегляне по траектория, loft, булеви операции и теселация на ACIS обекти.
- **GPU визуализация** — ускорени 2D и 3D изгледи чрез `wgpu`, с ортографски и перспективни камери.
- **Разширяеми работни процеси** — локални приставки, командни скриптове, преобразуване без графичен интерфейс и редово ориентиран JSON API за автоматизация.

<p align="center"><img src="../../site/modeling.png" alt="3D модел в Mac2CAM" width="100%"></p>

## Работа с файлове

| Формат или работен процес | Поддръжка |
| --- | --- |
| DWG | Четене и запис; целеви версии за запис от R14 до 2018 |
| DXF | Четене и запис; целеви версии за запис от R14 до 2018 |
| BAK / SV$ | Отваряне на резервни копия и автоматично записани файлове |
| OBJ | Импортиране на полигонални мрежи |
| LandXML | Импортиране на геодезически точки `CgPoint` |
| STL | Експортиране на данни за 3D мрежи |
| STEP AP203 | Експортиране на данни за 3D мрежи |
| PDF | Плотиране на оформления и избрана геометрия в настолната версия |
| CSV | Извличане на данни за свойствата на обектите |
| CTB / STB | Зареждане и редактиране на таблици със стилове за плотиране |

## Настолна или уеб версия

Използвайте [уеб приложението](https://www.mac2cam.com), за да започнете веднага без инсталиране. Чертежите се избират през браузъра и се записват като локални изтегляния.

Използвайте настолното приложение за локални файлови асоциации, миниатюри във файловия мениджър, системен печат, PDF изход, външни приставки, командни скриптове и автоматизация без графичен интерфейс. Предлагат се версии за Windows, Linux и macOS с Apple Silicon.

## Инсталиране

Изтеглете всички актуални пакети от [последната версия](https://github.com/HakanSeven12/Mac2CAM/releases/latest).

### Windows

Изберете един от подписаните x86-64 пакети:

- `Mac2CAM-*-windows-x86_64-installer.msi` — препоръчителна инсталационна програма с преки пътища в менюто Start, файлови асоциации за DWG/DXF и миниатюри на чертежите.
- `Mac2CAM-*-windows-x86_64-portable.exe` — самостоятелно приложение без необходимост от инсталиране.

### Linux

Изтеглете x86-64 AppImage файла, направете го изпълним и го стартирайте:

```bash
chmod +x Mac2CAM-*-linux-x86_64.AppImage
./Mac2CAM-*-linux-x86_64.AppImage
```

### macOS

Публикуваният пакет за macOS поддържа Apple Silicon:

1. Изтеглете `Mac2CAM-*-macos-arm64.dmg`.
2. Отворете образа и плъзнете `Mac2CAM.app` в папката **Applications**.
3. Ако Gatekeeper блокира първото стартиране, разрешете приложението от **System Settings → Privacy & Security**.

Приложението е подписано ad hoc, но към момента не е нотариално заверено от Apple.

## Езици

Mac2CAM може да следва системния език или да използва един от следните 21 езика на интерфейса:

> Арабски · Бразилски португалски · Български · Чешки · Нидерландски · Английски · Фински · Френски · Немски · Гръцки · Хинди · Унгарски · Италиански · Японски · Корейски · Полски · Руски · Опростен китайски · Испански · Традиционен китайски · Турски

Променете езика от настройките на приложението. Когато е избрано **Системен език**, браузърната версия също използва предпочитания локал на браузъра.

## Компилиране от изходния код

### Настолна версия

Изисквания:

- Git
- Актуална стабилна версия на инструментариума Rust
- Библиотеки за разработка на графика и шрифтове за съответната платформа

В Ubuntu или Debian инсталирайте локалните зависимости:

```bash
sudo apt update
sudo apt install libgl1-mesa-dev libx11-dev libxcursor-dev libxi-dev \
  libxrandr-dev libxkbcommon-dev libwayland-dev libfontconfig1-dev \
  libfreetype6-dev
```

След това компилирайте:

```bash
git clone https://github.com/HakanSeven12/Mac2CAM.git
cd Mac2CAM
cargo build --release --bin Mac2CAM
```

Полученият изпълним файл се записва в `target/release/Mac2CAM` (`Mac2CAM.exe` в Windows).

### Уеб версия

Инсталирайте еднократно WebAssembly целта и инструментите за компилиране:

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk wasm-bindgen-cli
```

Стартирайте сървъра за разработка:

```bash
trunk serve
```

## Автоматизация

Настолният изпълним файл поддържа еднократно преобразуване и постоянен сървър без графичен интерфейс:

```bash
Mac2CAM --export input.dwg output.dxf
Mac2CAM --serve
Mac2CAM --serve --port 4242
Mac2CAM --mcp
```

Сървърът обменя по един JSON обект на ред чрез стандартния вход/изход или локален TCP сокет. Вижте [ръководството за автоматизация](../automation/README.md).

## Приставки

Настолните приставки работят в отделни процеси и комуникират с основното приложение чрез версионирания API за приставки. Браузърната версия не зарежда локални приставки.

- [Архитектура на приставките](../plugin-architecture.md)
- [Шаблон за приставка](../plugin-template/README.md)
- [Регистър на приставките](../../plugins/README.md)

## Документация на проекта

- [API за автоматизация](../automation/README.md)
- [Архитектура на приставките](../plugin-architecture.md)
- [Процес на теселация](../tessellation.md)
- [Политика за сигурност](../../SECURITY.md)

## Принос

Приветстват се доклади за грешки, целенасочени pull request-и, преводи, подобрения на документацията и приноси към приставките.

- Потърсете в съществуващите [issues](https://github.com/HakanSeven12/Mac2CAM/issues), преди да отворите нов доклад.
- Използвайте [Discussions](https://github.com/HakanSeven12/Mac2CAM/discussions) за въпроси и идеи.
- Докладвайте уязвимости поверително, като следвате [политиката за сигурност](../../SECURITY.md).

## Развитие на проекта

<a href="https://github.com/HakanSeven12/Mac2CAM/stargazers">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://www.mac2cam.com/star-history-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="https://www.mac2cam.com/star-history-light.svg">
    <img alt="Звезди и изтегляния на Mac2CAM" src="https://www.mac2cam.com/star-history-light.svg">
  </picture>
</a>

## Подкрепете проекта

Ако Mac2CAM ви помага в работата, подкрепете по-нататъшното развитие чрез [GitHub Sponsors](https://github.com/sponsors/HakanSeven12) или [Patreon](https://www.patreon.com/HakanSeven12).

## Лиценз

Mac2CAM се разпространява съгласно [GNU General Public License v3.0](../../LICENSE).
