<?php

if ( ! defined( 'ABSPATH' ) ) {
    exit;
}

class Genossi_Beitritt_Form_Handler {

    private static $instance = null;

    private $errors = array();
    private $form_data = array();
    private $success = false;

    public static function init() {
        add_action( 'init', array( __CLASS__, 'handle_submission' ) );
    }

    public static function get_instance(): self {
        if ( null === self::$instance ) {
            self::$instance = new self();
        }
        return self::$instance;
    }

    public static function handle_submission() {
        if ( 'POST' !== $_SERVER['REQUEST_METHOD'] ) {
            return;
        }

        if ( ! isset( $_POST['genossi_beitritt_submit'] ) ) {
            return;
        }

        $handler = self::get_instance();
        $handler->process();
    }

    private function process() {
        // Collect form data
        $this->form_data = array(
            'salutation'   => sanitize_text_field( wp_unslash( $_POST['genossi_salutation'] ?? '' ) ),
            'first_name'   => sanitize_text_field( wp_unslash( $_POST['genossi_first_name'] ?? '' ) ),
            'last_name'    => sanitize_text_field( wp_unslash( $_POST['genossi_last_name'] ?? '' ) ),
            'email'        => sanitize_email( wp_unslash( $_POST['genossi_email'] ?? '' ) ),
            'street'       => sanitize_text_field( wp_unslash( $_POST['genossi_street'] ?? '' ) ),
            'house_number' => sanitize_text_field( wp_unslash( $_POST['genossi_house_number'] ?? '' ) ),
            'postal_code'  => sanitize_text_field( wp_unslash( $_POST['genossi_postal_code'] ?? '' ) ),
            'city'         => sanitize_text_field( wp_unslash( $_POST['genossi_city'] ?? '' ) ),
            'shares'       => absint( $_POST['genossi_shares'] ?? 1 ),
            'privacy'      => sanitize_text_field( $_POST['genossi_privacy'] ?? '' ),
            'statutes'     => sanitize_text_field( $_POST['genossi_statutes'] ?? '' ),
        );

        // Verify nonce
        if ( ! isset( $_POST['genossi_beitritt_nonce'] )
            || ! wp_verify_nonce( sanitize_text_field( wp_unslash( $_POST['genossi_beitritt_nonce'] ) ), 'genossi_beitritt_submit' ) ) {
            $this->errors[] = 'Sicherheitspruefung fehlgeschlagen. Bitte versuchen Sie es erneut.';
            return;
        }

        // Server-side validation
        $this->validate();

        if ( ! empty( $this->errors ) ) {
            return;
        }

        // Call Genossi API
        $this->call_api();
    }

    private function validate() {
        if ( empty( $this->form_data['first_name'] ) ) {
            $this->errors[] = 'Bitte geben Sie Ihren Vornamen ein.';
        }
        if ( empty( $this->form_data['last_name'] ) ) {
            $this->errors[] = 'Bitte geben Sie Ihren Nachnamen ein.';
        }
        if ( empty( $this->form_data['email'] ) || ! is_email( $this->form_data['email'] ) ) {
            $this->errors[] = 'Bitte geben Sie eine gueltige E-Mail-Adresse ein.';
        }
        if ( empty( $this->form_data['street'] ) ) {
            $this->errors[] = 'Bitte geben Sie Ihre Strasse ein.';
        }
        if ( empty( $this->form_data['house_number'] ) ) {
            $this->errors[] = 'Bitte geben Sie Ihre Hausnummer ein.';
        }
        if ( empty( $this->form_data['postal_code'] ) ) {
            $this->errors[] = 'Bitte geben Sie Ihre Postleitzahl ein.';
        }
        if ( empty( $this->form_data['city'] ) ) {
            $this->errors[] = 'Bitte geben Sie Ihren Ort ein.';
        }
        if ( $this->form_data['shares'] < 1 ) {
            $this->errors[] = 'Die Anzahl der Geschaeftsanteile muss mindestens 1 betragen.';
        }
        if ( empty( $this->form_data['privacy'] ) ) {
            $this->errors[] = 'Bitte akzeptieren Sie die Datenschutzerklaerung.';
        }
        if ( empty( $this->form_data['statutes'] ) ) {
            $this->errors[] = 'Bitte akzeptieren Sie die Satzung.';
        }
    }

    private function call_api() {
        $api_url = get_option( 'genossi_api_url', '' );
        $api_key = get_option( 'genossi_api_key', '' );

        $body = array(
            'first_name'   => $this->form_data['first_name'],
            'last_name'    => $this->form_data['last_name'],
            'email'        => $this->form_data['email'],
            'street'       => $this->form_data['street'],
            'house_number' => $this->form_data['house_number'],
            'postal_code'  => $this->form_data['postal_code'],
            'city'         => $this->form_data['city'],
            'shares'       => $this->form_data['shares'],
        );

        // Add salutation only if selected
        if ( ! empty( $this->form_data['salutation'] ) ) {
            $body['salutation'] = $this->form_data['salutation'];
        }

        $response = wp_remote_post(
            rtrim( $api_url, '/' ) . '/api/public/join',
            array(
                'headers' => array(
                    'Content-Type' => 'application/json',
                    'X-Api-Key'    => $api_key,
                ),
                'body'    => wp_json_encode( $body ),
                'timeout' => 30,
            )
        );

        if ( is_wp_error( $response ) ) {
            error_log( 'Genossi Beitritt API error: ' . $response->get_error_message() );
            $this->errors[] = 'Ein Fehler ist aufgetreten. Bitte versuchen Sie es spaeter erneut.';
            return;
        }

        $status_code = wp_remote_retrieve_response_code( $response );
        $response_body = wp_remote_retrieve_body( $response );

        if ( 201 === $status_code ) {
            $this->success = true;
            return;
        }

        if ( 422 === $status_code ) {
            $data = json_decode( $response_body, true );
            if ( isset( $data['message'] ) ) {
                $this->errors[] = $data['message'];
            } elseif ( isset( $data['errors'] ) && is_array( $data['errors'] ) ) {
                foreach ( $data['errors'] as $error ) {
                    $this->errors[] = is_string( $error ) ? $error : ( $error['message'] ?? 'Validierungsfehler' );
                }
            } else {
                $this->errors[] = 'Validierungsfehler bei der Verarbeitung Ihrer Daten.';
            }
            return;
        }

        if ( 401 === $status_code ) {
            error_log( 'Genossi Beitritt API: Unauthorized (401). Check API key configuration.' );
            $this->errors[] = 'Ein Fehler ist aufgetreten. Bitte versuchen Sie es spaeter erneut.';
            return;
        }

        error_log( 'Genossi Beitritt API: Unexpected status ' . $status_code . ' - Body: ' . $response_body );
        $this->errors[] = 'Ein Fehler ist aufgetreten. Bitte versuchen Sie es spaeter erneut.';
    }

    public function get_errors(): array {
        return $this->errors;
    }

    public function get_form_data(): array {
        return $this->form_data;
    }

    public function is_success(): bool {
        return $this->success;
    }
}
