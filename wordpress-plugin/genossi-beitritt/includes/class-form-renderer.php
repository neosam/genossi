<?php

if ( ! defined( 'ABSPATH' ) ) {
    exit;
}

class Genossi_Beitritt_Form_Renderer {

    public static function render( array $errors = array(), array $data = array() ) {
        ob_start();
        ?>
        <div class="genossi-beitritt-form-wrap">
            <?php if ( ! empty( $errors ) ) : ?>
                <div class="genossi-beitritt-errors">
                    <ul>
                        <?php foreach ( $errors as $error ) : ?>
                            <li><?php echo esc_html( $error ); ?></li>
                        <?php endforeach; ?>
                    </ul>
                </div>
            <?php endif; ?>

            <form method="post" class="genossi-beitritt-form">
                <?php wp_nonce_field( 'genossi_beitritt_submit', 'genossi_beitritt_nonce' ); ?>

                <div class="genossi-beitritt-field">
                    <label for="genossi_salutation">Anrede</label>
                    <select name="genossi_salutation" id="genossi_salutation">
                        <option value="">-- Bitte waehlen --</option>
                        <option value="Herr" <?php selected( self::val( $data, 'salutation' ), 'Herr' ); ?>>Herr</option>
                        <option value="Frau" <?php selected( self::val( $data, 'salutation' ), 'Frau' ); ?>>Frau</option>
                        <option value="Firma" <?php selected( self::val( $data, 'salutation' ), 'Firma' ); ?>>Firma</option>
                    </select>
                </div>

                <div class="genossi-beitritt-field">
                    <label for="genossi_first_name">Vorname <span class="required">*</span></label>
                    <input type="text" name="genossi_first_name" id="genossi_first_name"
                           value="<?php echo esc_attr( self::val( $data, 'first_name' ) ); ?>" required />
                </div>

                <div class="genossi-beitritt-field">
                    <label for="genossi_last_name">Nachname <span class="required">*</span></label>
                    <input type="text" name="genossi_last_name" id="genossi_last_name"
                           value="<?php echo esc_attr( self::val( $data, 'last_name' ) ); ?>" required />
                </div>

                <div class="genossi-beitritt-field">
                    <label for="genossi_email">E-Mail <span class="required">*</span></label>
                    <input type="email" name="genossi_email" id="genossi_email"
                           value="<?php echo esc_attr( self::val( $data, 'email' ) ); ?>" required />
                </div>

                <div class="genossi-beitritt-field genossi-beitritt-field-row">
                    <div class="genossi-beitritt-field-street">
                        <label for="genossi_street">Strasse <span class="required">*</span></label>
                        <input type="text" name="genossi_street" id="genossi_street"
                               value="<?php echo esc_attr( self::val( $data, 'street' ) ); ?>" required />
                    </div>
                    <div class="genossi-beitritt-field-house">
                        <label for="genossi_house_number">Nr. <span class="required">*</span></label>
                        <input type="text" name="genossi_house_number" id="genossi_house_number"
                               value="<?php echo esc_attr( self::val( $data, 'house_number' ) ); ?>" required />
                    </div>
                </div>

                <div class="genossi-beitritt-field genossi-beitritt-field-row">
                    <div class="genossi-beitritt-field-plz">
                        <label for="genossi_postal_code">PLZ <span class="required">*</span></label>
                        <input type="text" name="genossi_postal_code" id="genossi_postal_code"
                               value="<?php echo esc_attr( self::val( $data, 'postal_code' ) ); ?>" required />
                    </div>
                    <div class="genossi-beitritt-field-city">
                        <label for="genossi_city">Ort <span class="required">*</span></label>
                        <input type="text" name="genossi_city" id="genossi_city"
                               value="<?php echo esc_attr( self::val( $data, 'city' ) ); ?>" required />
                    </div>
                </div>

                <div class="genossi-beitritt-field">
                    <label for="genossi_shares">Anzahl Geschaeftsanteile <span class="required">*</span></label>
                    <input type="number" name="genossi_shares" id="genossi_shares"
                           value="<?php echo esc_attr( self::val( $data, 'shares', '1' ) ); ?>"
                           min="1" required />
                </div>

                <div class="genossi-beitritt-field genossi-beitritt-checkbox">
                    <label>
                        <input type="checkbox" name="genossi_privacy" value="1"
                               <?php checked( self::val( $data, 'privacy' ), '1' ); ?> required />
                        Ich habe die Datenschutzerklaerung gelesen und akzeptiert. <span class="required">*</span>
                    </label>
                </div>

                <div class="genossi-beitritt-field genossi-beitritt-checkbox">
                    <label>
                        <input type="checkbox" name="genossi_statutes" value="1"
                               <?php checked( self::val( $data, 'statutes' ), '1' ); ?> required />
                        Ich habe die Satzung gelesen und akzeptiert. <span class="required">*</span>
                    </label>
                </div>

                <div class="genossi-beitritt-submit">
                    <button type="submit" name="genossi_beitritt_submit" class="button">
                        Beitrittserklaerung absenden
                    </button>
                </div>
            </form>
        </div>
        <?php
        return ob_get_clean();
    }

    public static function render_success() {
        ob_start();
        ?>
        <div class="genossi-beitritt-success">
            <h3>Vielen Dank fuer Ihre Beitrittserklaerung!</h3>
            <p>Sie erhalten in Kuerze eine E-Mail mit den Ueberweisungsdaten.</p>
        </div>
        <?php
        return ob_get_clean();
    }

    private static function val( array $data, string $key, string $default = '' ): string {
        return isset( $data[ $key ] ) ? $data[ $key ] : $default;
    }
}
