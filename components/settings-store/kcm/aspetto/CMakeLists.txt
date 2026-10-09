kcmutils_add_qml_kcm(kcm_vela_aspetto SOURCES kcm_vela_aspetto.cpp)
target_compile_definitions(kcm_vela_aspetto PRIVATE TRANSLATION_DOMAIN="kcm_vela_aspetto")
target_link_libraries(kcm_vela_aspetto PRIVATE Qt6::Quick KF6::KCMUtilsQuick KF6::I18n KF6::CoreAddons)
