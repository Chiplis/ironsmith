# Build the upstream renderer as a reference executable, never as part of the
# browser application. This wrapper accommodates Homebrew's wx-config install;
# the pinned upstream CMake file expects a Windows/vcpkg CONFIG package.
# On macOS first run: node web/ui/scripts/mse-native/patch-macos.mjs MSE_SOURCE
# cmake -S web/ui/scripts/mse-native -B /tmp/mse-native-build \
#   -DMSE_SOURCE=/path/to/CajunAvenger-MagicSetEditor2 -DCMAKE_BUILD_TYPE=Release
# cmake --build /tmp/mse-native-build --target magicseteditor -j 2
# node web/ui/scripts/mse-native/prepare-resources.mjs BUILD MSE_SOURCE CORPUS
cmake_minimum_required(VERSION 3.20)
project(magicseteditor VERSION 2.6.4 LANGUAGES CXX)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(MSE_SOURCE "" CACHE PATH "Pinned upstream Magic Set Editor source checkout")
if(NOT EXISTS "${MSE_SOURCE}/src/main.cpp")
  message(FATAL_ERROR "MSE_SOURCE must point to the upstream MSE source checkout")
endif()
find_package(wxWidgets 3.3.1 REQUIRED COMPONENTS core base html net xml)
include(${wxWidgets_USE_FILE})
find_package(Boost CONFIG REQUIRED COMPONENTS regex)
find_package(PkgConfig REQUIRED)
pkg_check_modules(HUNSPELL REQUIRED IMPORTED_TARGET hunspell)
configure_file(${MSE_SOURCE}/src/config.hpp.in src/config.hpp)
file(GLOB_RECURSE sources ${MSE_SOURCE}/src/*.cpp)
list(FILTER sources EXCLUDE REGEX win32_cli_wrapper.cpp)
add_executable(magicseteditor ${sources})
target_compile_definitions(magicseteditor PRIVATE UNOFFICIAL_BUILD)
target_include_directories(magicseteditor PRIVATE ${MSE_SOURCE}/src ${CMAKE_BINARY_DIR}/src)
target_link_libraries(magicseteditor PRIVATE ${wxWidgets_LIBRARIES} Boost::regex PkgConfig::HUNSPELL)
if(APPLE)
  target_link_libraries(magicseteditor PRIVATE "-framework CoreText")
endif()
target_precompile_headers(magicseteditor PRIVATE ${MSE_SOURCE}/src/util/prec.hpp)
