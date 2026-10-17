# 開発で使うコマンド。docv/guide/develop.yaml に載せる。

DOCV := docv
TEMPLATE := assets/template/doc-guide.md

.PHONY: fix check-doc-guide install

# コードとドキュメントを整えて確かめ、HTML と JPG を作り直す。指摘が 1 件でもあれば止まる
# cargo test の後に install し、以降は install した docv で実行する
fix:
	cargo fmt
	cargo clippy --all-targets -- -D warnings
	cargo test
	@$(MAKE) --no-print-directory check-doc-guide
	$(DOCV) fix
	$(DOCV) check
	$(DOCV) build

# Doc ガイドの雛形は管理対象の外にあるので、一時ディレクトリに書き出して docv fix し、変わったら雛形に書き戻して install し直す
# そのうえで docv check を通す
check-doc-guide: install
	@set -e; tmp=$$(mktemp -d); trap 'rm -rf "$$tmp"' EXIT; \
	printf 'lint:\n  textlint: enable\nformat:\n  enabled: true\n' > "$$tmp/docv.yaml"; \
	(cd "$$tmp" && $(DOCV) slot gen doc-guide > /dev/null && $(DOCV) fix > /dev/null); \
	if ! cmp -s "$$tmp/docv/guide/doc-guide.md" $(TEMPLATE); then \
		cp "$$tmp/docv/guide/doc-guide.md" $(TEMPLATE); \
		echo "$(TEMPLATE) を整形した"; \
		cargo install -q --path .; \
	fi; \
	status=0; out=$$(cd "$$tmp" && $(DOCV) check) || status=$$?; \
	echo "$$out" | sed 's|^docv/guide/doc-guide.md|$(TEMPLATE)|'; \
	exit $$status

install:
	cargo install --path .
