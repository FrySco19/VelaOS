kcmutils_add_qml_kcm(kcm_vela_ai SOURCES kcm_vela_ai.cpp ../common/velaconfig.cpp ../common/velakcmbase.cpp)
target_include_directories(kcm_vela_ai PRIVATE ../common)
target_compile_definitions(kcm_vela_ai PRIVATE TRANSLATION_DOMAIN="kcm_vela_ai")
target_link_libraries(kcm_vela_ai PRIVATE Qt6::Quick Qt6::Network KF6::KCMUtilsQuick KF6::I18n KF6::CoreAddons)
