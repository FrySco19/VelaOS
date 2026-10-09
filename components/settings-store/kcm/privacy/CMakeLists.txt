kcmutils_add_qml_kcm(kcm_vela_privacy SOURCES kcm_vela_privacy.cpp ../common/velaconfig.cpp ../common/velakcmbase.cpp)
target_include_directories(kcm_vela_privacy PRIVATE ../common)
target_compile_definitions(kcm_vela_privacy PRIVATE TRANSLATION_DOMAIN="kcm_vela_privacy")
target_link_libraries(kcm_vela_privacy PRIVATE Qt6::Quick Qt6::Gui KF6::KCMUtilsQuick KF6::I18n KF6::CoreAddons)
