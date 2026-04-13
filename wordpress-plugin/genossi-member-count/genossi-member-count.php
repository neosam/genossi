<?php
/**
 * Plugin Name: Genossi Member Count
 * Plugin URI:  https://github.com/neosam/genossi3
 * Description: Zeigt die Anzahl aktiver Genossenschafts-Mitglieder per Shortcode an. Ruft die Genossi-API ab und cached das Ergebnis.
 * Version:     1.0.0
 * Author:      Genossi
 * License:     GPL-2.0-or-later
 * Text Domain: genossi-member-count
 * Requires PHP: 7.4
 */

if ( ! defined( 'ABSPATH' ) ) {
    exit;
}

define( 'GENOSSI_MC_VERSION', '1.0.0' );
define( 'GENOSSI_MC_PLUGIN_DIR', plugin_dir_path( __FILE__ ) );

require_once GENOSSI_MC_PLUGIN_DIR . 'includes/class-settings.php';

// Initialize settings
Genossi_Member_Count_Settings::init();

// Register shortcode
add_shortcode( 'genossi_member_count', 'genossi_member_count_shortcode' );

/**
 * Resolve the API base URL using fallback chain.
 *
 * @return string API base URL or empty string if not configured.
 */
function genossi_mc_get_api_url() {
    $url = get_option( 'genossi_api_url', '' );
    if ( empty( $url ) ) {
        $url = get_option( 'genossi_mc_api_url', '' );
    }
    return $url;
}

/**
 * Fetch the member count from cache or API.
 *
 * @param string $api_url The API base URL.
 * @return int|false Member count or false on failure.
 */
function genossi_mc_get_count( $api_url ) {
    $cached = get_transient( 'genossi_member_count' );
    if ( false !== $cached ) {
        return (int) $cached;
    }

    $endpoint = rtrim( $api_url, '/' ) . '/api/public/member-count';
    $response = wp_remote_get( $endpoint, array( 'timeout' => 10 ) );

    if ( is_wp_error( $response ) ) {
        return false;
    }

    $status_code = wp_remote_retrieve_response_code( $response );
    if ( 200 !== $status_code ) {
        return false;
    }

    $body = json_decode( wp_remote_retrieve_body( $response ), true );
    if ( ! is_array( $body ) || ! isset( $body['count'] ) ) {
        return false;
    }

    $count = (int) $body['count'];
    $ttl = (int) get_option( 'genossi_mc_cache_ttl', 900 );
    set_transient( 'genossi_member_count', $count, $ttl );

    return $count;
}

/**
 * Shortcode handler for [genossi_member_count].
 *
 * @return string HTML output.
 */
function genossi_member_count_shortcode() {
    $api_url = genossi_mc_get_api_url();

    if ( empty( $api_url ) ) {
        if ( current_user_can( 'manage_options' ) ) {
            return '<span class="genossi-member-count-notice">'
                . '<strong>Genossi Member Count:</strong> Plugin nicht konfiguriert. '
                . '<a href="' . esc_url( admin_url( 'options-general.php?page=genossi-member-count' ) ) . '">Bitte API-URL einstellen</a>.'
                . '</span>';
        }
        return '';
    }

    $count = genossi_mc_get_count( $api_url );

    if ( false === $count ) {
        return '';
    }

    return '<span class="genossi-member-count">' . esc_html( $count ) . '</span>';
}
