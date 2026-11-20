# 開発で使うコマンド。docv/slot/guide/develop.md に載せる。

DOCV := docv
# docv gen が書く雛形（assets/template/）の Slot
TEMPLATES := doc-guide words stack env cost install setup develop

.PHONY: fix check-templates install new-version

# コードとドキュメントを整えて確かめ、HTML を作り直す。指摘が 1 件でもあれば止まる
# cargo test の後に install し、以降は install した docv で実行する
fix:
	cargo fmt
	cargo clippy --all-targets -- -D warnings
	cargo test
	@$(MAKE) --no-print-directory check-templates
	$(DOCV) fix
	$(DOCV) check
	$(DOCV) build

# 雛形は管理対象の外にあるので、一時ディレクトリに docv gen で書き出して docv fix し、変わったら雛形に書き戻して install し直す
# そのうえで docv check を通す
check-templates: install
	@set -e; tmp=$$(mktemp -d); trap 'rm -rf "$$tmp"' EXIT; \
	printf 'docv_version: %s\nlint:\n  textlint: enable\nformat:\n  enabled: true\n' "$$($(DOCV) --version | cut -d ' ' -f 2)" > "$$tmp/docv.yaml"; \
	(cd "$$tmp" && for slot in $(TEMPLATES); do $(DOCV) gen $$slot > /dev/null; done && $(DOCV) fix > /dev/null); \
	changed=0; \
	for slot in $(TEMPLATES); do \
		generated=$$(cd "$$tmp" && find docv/slot -name "$$slot.md"); \
		if ! cmp -s "$$tmp/$$generated" assets/template/$$slot.md; then \
			cp "$$tmp/$$generated" assets/template/$$slot.md; \
			echo "assets/template/$$slot.md を整形した"; \
			changed=1; \
		fi; \
	done; \
	if [ $$changed = 1 ]; then cargo install -q --path .; fi; \
	status=0; out=$$(cd "$$tmp" && $(DOCV) check) || status=$$?; \
	echo "$$out" | sed -E 's|^docv/slot/[a-z]+/([a-z-]+)\.md|assets/template/\1.md|'; \
	exit $$status

install:
	cargo install --path .

# 版を上げる。人間が実行する。版の書き換え、make fix、コミット、タグ、push までを行う（docv/slot/operation/release.md）
# 例: make new-version V=x.y.z
new-version:
	@scripts/new-version.sh "$(V)"
