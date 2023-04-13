.PHONY: doc

doc:
	cargo doc -p node_driver --serve

book:
	mdbook test tutorial && mdbook serve tutorial
