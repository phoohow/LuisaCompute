target("stb-image")
set_basename("lc-ext-stb-image")
_config_project({
	project_kind = "shared"
})
add_files("../ext/stb/stb.c")
add_includedirs("../ext/stb", {
	public = true
})
target_end()
local function test_proj(name)
	target(name)
	_config_project({
		project_kind = "binary"
	})
	add_files(name .. ".cpp")
	add_deps("lc-runtime", "lc-vstl", "lc-gui", "stb-image", "lc-backends-dummy")
	if get_config("enable_dsl") then
		add_deps("lc-dsl")
	end
	target_end()
end

-- FIXME: @Maxwell please use the doctest framework
test_proj("test_helloworld")
test_proj("test_ast")
test_proj("test_atomic")
test_proj("test_bindless")
test_proj("test_callable")
--test_proj("test_dsl")
test_proj("test_dsl_multithread")
test_proj("test_dsl_sugar")
test_proj("test_game_of_life")
test_proj("test_mpm3d")
test_proj("test_mpm88")
test_proj("test_normal_encoding")
test_proj("test_path_tracing")
test_proj("test_path_tracing_camera")
test_proj("test_path_tracing_cutout")
test_proj("test_photon_mapping")
test_proj("test_printer")
test_proj("test_procedural")
test_proj("test_rtx")
test_proj("test_runtime")
test_proj("test_sampler")
test_proj("test_sdf_renderer")
add_defines("ENABLE_DISPLAY")
test_proj("test_shader_toy")
test_proj("test_shader_visuals_present")
test_proj("test_texture_io")
test_proj("test_thread_pool")
test_proj("test_type")
test_proj("test_raster")
test_proj("test_texture_compress")
test_proj("test_swapchain")
test_proj("test_swapchain_static")
