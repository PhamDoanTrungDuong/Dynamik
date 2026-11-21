# Dynamik — My Notch

A Dynamic Island–style notch for Windows 10/11, written in Rust (egui). It sits at the top edge of the
screen, shows what's going on, and opens into a small panel of tools.

*Một "notch" kiểu Dynamic Island cho Windows 10/11, viết bằng Rust. Notch nằm ở mép trên màn hình,
hiện trạng thái và mở ra thành bảng công cụ nhỏ.*

![My Notch](app/shots/now.png)

## Tính năng

**Notch**
- Notch nghỉ có mascot: đôi mắt chớp mắt, dải sáng xanh–tím bo theo góc notch.
- Rê chuột (hoặc click) để mở; click phải để vào Settings; ẩn khi có game, video, slide toàn màn hình.
- Icon khay hệ thống: Start with Windows, Mascot, Minimal notch, About, Exit.

**Các trang** (sắp xếp lại hoặc ẩn trong Settings › Pages)
- **Music**: bài đang phát từ mọi trình phát (Spotify, trình duyệt…), play/pause, chuyển bài, tua;
  **lời bài hát chạy theo nhạc** từ LRCLIB, click một dòng để nhảy tới.
- **Pomodoro**: đồng hồ tập trung, đếm vòng, nghỉ ngắn/dài, pop-up khi hết giờ.
- **Water**: nhắc uống nước theo khung giờ, dung tích bình, mục tiêu và biểu đồ theo giờ.
- **Shelf**: kéo file vào notch, kéo ra thư mục hoặc ô chat; lịch sử clipboard.
- **To-do**: việc có giờ, nhắc khi bắt đầu và khi hết giờ, nút "Later".
- **Prayer**: giờ cầu nguyện trên vòng cung mặt trời, nhắc giờ, tự dừng nhạc.
- **Widgets**: 2 widget một lúc — tuần, pin, giờ cầu nguyện, focus, giờ thế giới, mặt trăng, đếm ngược,
  nước, thời tiết.
- **Today**: thông báo gần đây và tuần của bạn.

**Pop-up hệ thống**: âm lượng, độ sáng, Caps Lock, ngôn ngữ bàn phím, pin, Wi-Fi, tai nghe Bluetooth,
thông báo Windows.

**Công cụ nhanh**
- `Ctrl + Alt + Space`: ô hỏi nhanh — tính toán (`12*(3+4)=`) và đổi tiền (`100usd to vnd=`).
- `Ctrl + Alt + T`: OCR — kéo chọn một vùng màn hình, chữ trong đó được đọc (offline) và copy.

**Ngôn ngữ**: English, Tiếng Việt, 中文 (简体), Français.

## Cài đặt

1. Tải `MyNotch-<version>-windows-x64.zip` ở trang [Releases](https://github.com/PhamDoanTrungDuong/Dynamik/releases).
2. Giải nén, bấm đúp `install.cmd` (không cần quyền admin). App được chép vào
   `%LOCALAPPDATA%\Programs\MyNotch` và có lối tắt "My Notch" trong Start Menu.
3. Nếu Windows hiện "Windows protected your PC": bấm **More info → Run anyway** (app chưa có chữ ký số).

Gỡ cài đặt: chạy `%LOCALAPPDATA%\Programs\MyNotch\uninstall.cmd`. Cài đặt riêng nằm ở
`%LOCALAPPDATA%\MyNotch`.

## Build từ mã nguồn

Cần [Rust](https://rustup.rs) (stable, target `x86_64-pc-windows-msvc`).

```sh
cd app
cargo build --release      # app/target/release/notch.exe
cargo test --release
```

Icon của exe được nhúng bằng `windres` (ví dụ từ LLVM-MinGW) nếu có trong PATH; không có thì exe vẫn
build được, chỉ thiếu icon. Các mẫu icon nằm trong `icon-options/`.

## Mạng và quyền riêng tư

App chạy hoàn toàn trên máy, trừ các tính năng sau (đều miễn phí, không cần khóa, chỉ gọi khi dùng):
- Lời bài hát: [LRCLIB](https://lrclib.net) (gửi tên bài và ca sĩ).
- Thời tiết: [MET Norway](https://api.met.no) (gửi tọa độ thành phố đã chọn).
- Tỉ giá: [open.er-api.com](https://open.er-api.com).

Để hiện pop-up Caps Lock / đổi ngôn ngữ bàn phím, app theo dõi các phím đó (chỉ xem mã phím, không ghi
lại gì); một số phần mềm diệt virus có thể cảnh báo vì việc này.

## Ghi chú

Đây là bản dựng lại độc lập, lấy cảm hứng từ app Nusa; không liên kết với tác giả của Nusa.
`app/MECHANICS.md` ghi lại cách từng tính năng hoạt động.
