
.const DDRB = $dc03
.const TALO = $dc04
.const TAHI = $dc05
.const TBLO = $dc06
.const TBHI = $dc07
.const ICR = $dc0d
.const CRA = $dc0e
.const CRB = $dcef

			* = $4000 "Main Program"		// <- The name 'Main program' will appear in the memory map when assembling
start:		sei
            lda #$7f
            sta DDRB
            lda #$82
            sta ICR
            lda #$00
            sta CRA
            sta CRB
            lda #$02
            sta TALO
            sta TBLO
            lda #$00
            sta TAHI
            sta TBHI
            lda #$47
            sta CRB
            lda #$03
            sta CRA
            jmp *
