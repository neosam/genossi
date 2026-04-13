<?php
/**
 * Plugin Name: Genossi Beitritt
 * Plugin URI:  https://github.com/neosam/genossi3
 * Description: Beitrittsformular fuer die Genossenschaft. Rendert ein Formular per Shortcode und sendet die Daten serverseitig an die Genossi-API.
 * Version:     1.0.0
 * Author:      Genossi
 * License:     GPL-2.0-or-later
 * Text Domain: genossi-beitritt
 * Requires PHP: 7.4
 */

if ( ! defined( 'ABSPATH' ) ) {
    exit;
}

define( 'GENOSSI_BEITRITT_VERSION', '1.0.0' );
define( 'GENOSSI_BEITRITT_PLUGIN_DIR', plugin_dir_path( __FILE__ ) );
define( 'GENOSSI_BEITRITT_PLUGIN_URL', plugin_dir_url( __FILE__ ) );

require_once GENOSSI_BEITRITT_PLUGIN_DIR . 'includes/class-settings.php';
require_once GENOSSI_BEITRITT_PLUGIN_DIR . 'includes/class-form-renderer.php';
require_once GENOSSI_BEITRITT_PLUGIN_DIR . 'includes/class-form-handler.php';

// Initialize settings
Genossi_Beitritt_Settings::init();

// Initialize form handler (processes POST submissions)
Genossi_Beitritt_Form_Handler::init();

// Register shortcode
add_shortcode( 'genossi_beitritt', 'genossi_beitritt_shortcode' );

function genossi_beitritt_shortcode() {
    $api_url = get_option( 'genossi_api_url', '' );
    $api_key = get_option( 'genossi_api_key', '' );

    if ( empty( $api_url ) || empty( $api_key ) ) {
        if ( current_user_can( 'manage_options' ) ) {
            return '<div class="genossi-beitritt-notice">'
                . '<p><strong>Genossi Beitritt:</strong> Plugin nicht konfiguriert. '
                . 'Bitte <a href="' . esc_url( admin_url( 'options-general.php?page=genossi-beitritt' ) ) . '">API-URL und API-Key einstellen</a>.</p>'
                . '</div>';
        }
        return '';
    }

    // Enqueue CSS
    wp_enqueue_style(
        'genossi-beitritt',
        GENOSSI_BEITRITT_PLUGIN_URL . 'assets/style.css',
        array(),
        GENOSSI_BEITRITT_VERSION
    );

    $handler = Genossi_Beitritt_Form_Handler::get_instance();

    if ( $handler->is_success() ) {
        return Genossi_Beitritt_Form_Renderer::render_success();
    }

    return Genossi_Beitritt_Form_Renderer::render(
        $handler->get_errors(),
        $handler->get_form_data()
    );
}
