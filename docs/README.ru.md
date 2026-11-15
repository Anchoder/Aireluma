<div align="center">
  <picture><source media="(prefers-reduced-motion: reduce)" srcset="images/banner-readable.png"><img src="images/banner-readable.gif" alt="Aireluma — экран iPhone на Windows и Linux" width="100%"></picture>
  <p><strong>Экран и звук iPhone на компьютере. Без проводов.</strong></p>
  <p><a href="https://github.com/Anchoder/Aireluma/releases/latest/download/Aireluma-Setup.exe">Windows</a> · <a href="https://github.com/Anchoder/Aireluma/releases/download/v0.8.3/Aireluma_0.8.3_amd64.deb">Linux .deb</a> · <a href="../README.md">English</a> · <a href="https://github.com/Anchoder/Aireluma/issues">Сообщить о проблеме</a></p>
</div>

## Возможности

- Повтор экрана и звука iPhone по AirPlay в локальной сети.
- Приём с запросом HEVC, разрешения до 4K и частоты до 60 FPS.
- Светлая и тёмная темы с плавным переходом и сохранением выбора.
- Интерфейс на русском, английском и немецком: язык переключается в окне (пока только в версии для Windows).
- Работа в трее после закрытия окна на Windows.
- Автоматическое окно просмотра: Direct3D12 на Windows, GStreamer на Linux.
- Обычный установщик и удаление через параметры Windows.

Фактическое разрешение, кодек и частоту выбирает iPhone. Результат зависит от устройства, сети и компьютера. Некоторые приложения ограничивают захват или используют отдельный маршрут AirPlay для видео; такой контент может не отображаться.

<div align="center"><picture><source media="(prefers-reduced-motion: reduce)" srcset="images/themes.ru.png"><img src="images/interface-tour.ru.gif" alt="Смена темы и языка, запуск приёма и ожидание iPhone" width="380"></picture></div>

<details><summary>Обе темы рядом</summary><img src="images/themes.ru.png" alt="Светлая и тёмная темы" width="100%"></details>

<details><summary>Три языка интерфейса</summary><img src="images/languages.png" alt="Интерфейс на английском, немецком и русском" width="100%"></details>

## Установка на Linux

Готовый пакет рассчитан на **Ubuntu 24.04 x64**. Скачайте `.deb` из [релиза](https://github.com/Anchoder/Aireluma/releases/latest) и установите:

```sh
sudo apt install ./Aireluma_0.8.3_amd64.deb
```

Приложение появится в меню программ. Удаление: `sudo apt remove aireluma`. Сборка для других дистрибутивов описана в [BUILDING.md](BUILDING.md). На Linux закрытие окна останавливает приёмник и завершает приложение.

## Подключение

1. Скачайте установщик из [последнего релиза](https://github.com/Anchoder/Aireluma/releases/latest).
2. Установите Aireluma, запустите её и нажмите **«Включить AirPlay»** (в английском интерфейсе — **Start AirPlay**, в немецком — **AirPlay starten**; язык выбирается внизу окна).
3. Подключите iPhone и компьютер к одной локальной сети. Компьютер может использовать Ethernet.
4. На iPhone откройте **Пункт управления → Повтор экрана** и выберите **Aireluma**.

Для Windows нужны версия 10/11 x64 и поддержка Direct3D12. Если приёмник не виден, проверьте, что устройства находятся в одной сети и роутер не изолирует клиентов. При разрешении доступа брандмауэра выбирайте только доверенную домашнюю сеть.

## Исходники

Интерфейс написан на Rust: Win32 API для Windows и GTK4 для Linux. Приём обеспечивает [UxPlay](https://github.com/FDH2/UxPlay), воспроизведение — GStreamer. Инструкция сборки: [BUILDING.md](BUILDING.md). Установщик опубликован в релизах; большие двоичные зависимости не хранятся в Git.

Проект распространяется под [GPL-3.0-or-later](../LICENSE). Лицензии сторонних компонентов сохраняются. Aireluma — независимый проект, не связанный с Apple.
