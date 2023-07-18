target("lc-rust")
_config_project({
	project_kind = "object"
})
before_build(function(target)
	os.addenvs({
		LC_RS_DO_NOT_GENERATE_BINDING = 1
	})
end)
add_rules("build_cargo")
add_files("Cargo.toml")
add_files("luisa_compute_ir/Cargo.toml")
-- add_files("luisa_compute_backend/Cargo.toml")
-- add_files("luisa_compute_backend_impl/Cargo.toml")
-- add_files("luisa_compute_cpu_kernel_defs/Cargo.toml")
target_end()
