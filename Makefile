PREFIX ?= /usr/local
BINDIR = $(PREFIX)/bin
DATADIR = $(PREFIX)/share
APPDIR = $(DATADIR)/applications
ICONDIR = $(DATADIR)/icons/hicolor

APP_NAME = vmedia-player
APP_ID = io.vmedia.native-player

.PHONY: build release install uninstall clean package package-deb

build:
	cargo build

release:
	cargo build --release

install: release
	@echo "Installing $(APP_NAME)..."
	install -Dm755 target/release/native-player $(DESTDIR)$(BINDIR)/$(APP_NAME)
	install -Dm644 resources/$(APP_ID).desktop $(DESTDIR)$(APPDIR)/$(APP_ID).desktop
	install -Dm644 resources/icons/hicolor/256x256/apps/vmedia.png $(DESTDIR)$(ICONDIR)/256x256/apps/vmedia.png
	install -Dm644 resources/icons/hicolor/128x128/apps/vmedia.png $(DESTDIR)$(ICONDIR)/128x128/apps/vmedia.png
	install -Dm644 resources/icons/hicolor/64x64/apps/vmedia.png $(DESTDIR)$(ICONDIR)/64x64/apps/vmedia.png
	install -Dm644 resources/icons/hicolor/48x48/apps/vmedia.png $(DESTDIR)$(ICONDIR)/48x48/apps/vmedia.png
	install -Dm644 resources/icons/hicolor/256x256/apps/vmedia.png $(DESTDIR)$(ICONDIR)/256x256/apps/$(APP_ID).png
	install -Dm644 resources/icons/hicolor/128x128/apps/vmedia.png $(DESTDIR)$(ICONDIR)/128x128/apps/$(APP_ID).png
	install -Dm644 resources/icons/hicolor/64x64/apps/vmedia.png $(DESTDIR)$(ICONDIR)/64x64/apps/$(APP_ID).png
	install -Dm644 resources/icons/hicolor/48x48/apps/vmedia.png $(DESTDIR)$(ICONDIR)/48x48/apps/$(APP_ID).png
	-gtk-update-icon-cache -f -t $(DESTDIR)$(ICONDIR) 2>/dev/null || true
	-update-desktop-database $(DESTDIR)$(APPDIR) 2>/dev/null || true
	@echo "Installed successfully! Run '$(APP_NAME)' to start."

uninstall:
	@echo "Uninstalling $(APP_NAME)..."
	rm -f $(DESTDIR)$(BINDIR)/$(APP_NAME)
	rm -f $(DESTDIR)$(APPDIR)/$(APP_ID).desktop
	rm -f $(DESTDIR)$(ICONDIR)/256x256/apps/vmedia.png
	rm -f $(DESTDIR)$(ICONDIR)/128x128/apps/vmedia.png
	rm -f $(DESTDIR)$(ICONDIR)/64x64/apps/vmedia.png
	rm -f $(DESTDIR)$(ICONDIR)/48x48/apps/vmedia.png
	rm -f $(DESTDIR)$(ICONDIR)/256x256/apps/$(APP_ID).png
	rm -f $(DESTDIR)$(ICONDIR)/128x128/apps/$(APP_ID).png
	rm -f $(DESTDIR)$(ICONDIR)/64x64/apps/$(APP_ID).png
	rm -f $(DESTDIR)$(ICONDIR)/48x48/apps/$(APP_ID).png
	-gtk-update-icon-cache -f -t $(DESTDIR)$(ICONDIR) 2>/dev/null || true
	-update-desktop-database $(DESTDIR)$(APPDIR) 2>/dev/null || true
	@echo "Uninstalled."

clean:
	cargo clean

package: release
	@echo "Creating release package..."
	$(eval PKG_DIR := release-pkg/vmedia-native-player-$(shell cargo metadata --no-deps --format-version 1 2>/dev/null | grep -o '"version":"[^"]*"' | head -1 | cut -d'"' -f4)-linux-x86_64)
	rm -rf $(PKG_DIR)
	mkdir -p $(PKG_DIR)/icons/hicolor/256x256/apps
	mkdir -p $(PKG_DIR)/icons/hicolor/128x128/apps
	mkdir -p $(PKG_DIR)/icons/hicolor/64x64/apps
	mkdir -p $(PKG_DIR)/icons/hicolor/48x48/apps
	cp target/release/native-player $(PKG_DIR)/vmedia-player
	cp resources/$(APP_ID).desktop $(PKG_DIR)/
	cp resources/icons/hicolor/256x256/apps/vmedia.png $(PKG_DIR)/icons/hicolor/256x256/apps/
	cp resources/icons/hicolor/128x128/apps/vmedia.png $(PKG_DIR)/icons/hicolor/128x128/apps/
	cp resources/icons/hicolor/64x64/apps/vmedia.png $(PKG_DIR)/icons/hicolor/64x64/apps/
	cp resources/icons/hicolor/48x48/apps/vmedia.png $(PKG_DIR)/icons/hicolor/48x48/apps/
	cp resources/icons/hicolor/256x256/apps/vmedia.png $(PKG_DIR)/icons/hicolor/256x256/apps/$(APP_ID).png
	cp resources/icons/hicolor/128x128/apps/vmedia.png $(PKG_DIR)/icons/hicolor/128x128/apps/$(APP_ID).png
	cp resources/icons/hicolor/64x64/apps/vmedia.png $(PKG_DIR)/icons/hicolor/64x64/apps/$(APP_ID).png
	cp resources/icons/hicolor/48x48/apps/vmedia.png $(PKG_DIR)/icons/hicolor/48x48/apps/$(APP_ID).png
	cp README.md $(PKG_DIR)/
	cp README_ZH.md $(PKG_DIR)/
	cp LICENSE $(PKG_DIR)/
	cd release-pkg && tar czf $(notdir $(PKG_DIR)).tar.gz $(notdir $(PKG_DIR))
	@echo "Package created: $(PKG_DIR).tar.gz"

package-deb: release
	@echo "Creating Debian package..."
	./scripts/build-deb.sh
