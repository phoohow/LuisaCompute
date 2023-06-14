#!/bin/bash

export CIBW_ARCHS=auto64
export CIBW_MANYLINUX_X86_64_IMAGE=manylinux_2_28
export CIBW_BEFORE_ALL="./scripts/cibw_install_deps.sh"
export CIBW_BUILD_VERBOSITY=2

cibuildwheel --output-dir wheelhouse --platform linux
