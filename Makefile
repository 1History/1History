release-arm64:
	./release-arm64.sh

serve:
	cargo run -- serve

publish:
	cargo publish --registry github --token $(CARGO_REGISTRY_TOKEN)
