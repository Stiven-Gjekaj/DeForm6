; rot47 by Jigsy (https://github.com/Jigsy1) released under The Unlicense.
;
; A simple Caesar cipher in one line of code.
;
; To load, simply: /load -rs "P:\ath\to\rot47.mrc"
;
; How to:
; ----------
; //say $rot47(Hello.)
; //var %x = My $!tring, with #characters % $+ mIRC doesn't like; goes here. | //say $rot47(%x)
; //echo -aet $rot47(#F336C 323J 3F88J 3F>A6CD])


alias rot47 { return $regsubex($1-,/(.)/g,$chr($iif($asc(\t) isnum 33-126,$calc(33 + ($v1 + 14) % 94),$v1))) }

; EOF
