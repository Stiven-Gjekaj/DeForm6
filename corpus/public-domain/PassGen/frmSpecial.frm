VERSION 5.00
Begin VB.Form frmSpecial 
   BorderStyle     =   1  'Fixed Single
   Caption         =   "Include Special characters"
   ClientHeight    =   3000
   ClientLeft      =   450
   ClientTop       =   540
   ClientWidth     =   6480
   KeyPreview      =   -1  'True
   LinkTopic       =   "Form1"
   MaxButton       =   0   'False
   ScaleHeight     =   3000
   ScaleWidth      =   6480
   StartUpPosition =   1  'CenterOwner
   Begin VB.CommandButton cmdExclude 
      Caption         =   "&Exclude All"
      Height          =   375
      Left            =   1560
      TabIndex        =   33
      Top             =   2520
      Width           =   1335
   End
   Begin VB.CommandButton cmdIncludeAll 
      Caption         =   "&Include All"
      Height          =   375
      Left            =   120
      TabIndex        =   32
      Top             =   2520
      Width           =   1335
   End
   Begin VB.CommandButton cmdOkay 
      Caption         =   "&OK"
      Height          =   375
      Left            =   5040
      TabIndex        =   34
      Top             =   2520
      Width           =   1335
   End
   Begin VB.Frame fmeInclude 
      Caption         =   "Include:"
      Height          =   2295
      Left            =   120
      TabIndex        =   35
      Top             =   120
      Width           =   6255
      Begin VB.CheckBox chkUnderscore 
         Caption         =   "_"
         Height          =   255
         Left            =   4560
         TabIndex        =   26
         Top             =   720
         Value           =   1  'Checked
         Width           =   1575
      End
      Begin VB.CheckBox chkQuote 
         Caption         =   """"
         Height          =   255
         Left            =   240
         TabIndex        =   1
         Top             =   480
         Value           =   1  'Checked
         Width           =   1335
      End
      Begin VB.CheckBox chkRightBrace 
         Caption         =   "}"
         Height          =   255
         Left            =   4560
         TabIndex        =   30
         Top             =   1680
         Value           =   1  'Checked
         Width           =   1575
      End
      Begin VB.CheckBox chkLeftBrace 
         Caption         =   "{"
         Height          =   255
         Left            =   4560
         TabIndex        =   28
         Top             =   1200
         Value           =   1  'Checked
         Width           =   1575
      End
      Begin VB.CheckBox chkRightBracket 
         Caption         =   "]"
         Height          =   255
         Left            =   4560
         TabIndex        =   24
         Top             =   240
         Value           =   1  'Checked
         Width           =   1575
      End
      Begin VB.CheckBox chkLeftBracket 
         Caption         =   "["
         Height          =   255
         Left            =   3120
         TabIndex        =   22
         Top             =   1680
         Value           =   1  'Checked
         Width           =   1335
      End
      Begin VB.CheckBox chkRightParenthesis 
         Caption         =   ")"
         Height          =   255
         Left            =   1680
         TabIndex        =   8
         Top             =   240
         Value           =   1  'Checked
         Width           =   1335
      End
      Begin VB.CheckBox chkLeftParenthesis 
         Caption         =   "("
         Height          =   255
         Left            =   240
         TabIndex        =   7
         Top             =   1920
         Value           =   1  'Checked
         Width           =   1335
      End
      Begin VB.CheckBox chkPipe 
         Caption         =   "|"
         Height          =   255
         Left            =   4560
         TabIndex        =   29
         Top             =   1440
         Value           =   1  'Checked
         Width           =   1575
      End
      Begin VB.CheckBox chkComma 
         Caption         =   ", (Comma)"
         Height          =   255
         Left            =   1680
         TabIndex        =   11
         Top             =   960
         Value           =   1  'Checked
         Width           =   1335
      End
      Begin VB.CheckBox chkPercent 
         Caption         =   "%"
         Height          =   255
         Left            =   240
         TabIndex        =   4
         Top             =   1200
         Value           =   1  'Checked
         Width           =   1335
      End
      Begin VB.CheckBox chkDollar 
         Caption         =   "$"
         Height          =   255
         Left            =   240
         TabIndex        =   3
         Top             =   960
         Value           =   1  'Checked
         Width           =   1335
      End
      Begin VB.CheckBox chkHash 
         Caption         =   "#"
         Height          =   255
         Left            =   240
         TabIndex        =   2
         Top             =   720
         Value           =   1  'Checked
         Width           =   1335
      End
      Begin VB.CheckBox chkTilde 
         Caption         =   "~"
         Height          =   255
         Left            =   4560
         TabIndex        =   31
         Top             =   1920
         Value           =   1  'Checked
         Width           =   1575
      End
      Begin VB.CheckBox chkGrave 
         Caption         =   "` (Grave Accent)"
         Height          =   255
         Left            =   4560
         TabIndex        =   27
         Top             =   960
         Value           =   1  'Checked
         Width           =   1575
      End
      Begin VB.CheckBox chkPower 
         Caption         =   "^"
         Height          =   255
         Left            =   4560
         TabIndex        =   25
         Top             =   480
         Value           =   1  'Checked
         Width           =   1575
      End
      Begin VB.CheckBox chkBackSlash 
         Caption         =   "\"
         Height          =   255
         Left            =   3120
         TabIndex        =   23
         Top             =   1920
         Value           =   1  'Checked
         Width           =   1335
      End
      Begin VB.CheckBox chkAtSign 
         Caption         =   "@"
         Height          =   255
         Left            =   3120
         TabIndex        =   21
         Top             =   1440
         Value           =   1  'Checked
         Width           =   1335
      End
      Begin VB.CheckBox chkQuestion 
         Caption         =   "?"
         Height          =   255
         Left            =   3120
         TabIndex        =   20
         Top             =   1200
         Value           =   1  'Checked
         Width           =   1335
      End
      Begin VB.CheckBox chkGreaterThan 
         Caption         =   ">"
         Height          =   255
         Left            =   3120
         TabIndex        =   19
         Top             =   960
         Value           =   1  'Checked
         Width           =   1335
      End
      Begin VB.CheckBox chkEquals 
         Caption         =   "="
         Height          =   255
         Left            =   3120
         TabIndex        =   18
         Top             =   720
         Value           =   1  'Checked
         Width           =   1335
      End
      Begin VB.CheckBox chkLessThan 
         Caption         =   "<"
         Height          =   255
         Left            =   3120
         TabIndex        =   17
         Top             =   480
         Value           =   1  'Checked
         Width           =   1335
      End
      Begin VB.CheckBox chkSemiColon 
         Caption         =   "; (Semicolon)"
         Height          =   255
         Left            =   3120
         TabIndex        =   16
         Top             =   240
         Value           =   1  'Checked
         Width           =   1335
      End
      Begin VB.CheckBox chkColon 
         Caption         =   ": (Colon)"
         Height          =   255
         Left            =   1680
         TabIndex        =   15
         Top             =   1920
         Value           =   1  'Checked
         Width           =   1335
      End
      Begin VB.CheckBox chkForwardSlash 
         Caption         =   "/"
         Height          =   255
         Left            =   1680
         TabIndex        =   14
         Top             =   1680
         Value           =   1  'Checked
         Width           =   1335
      End
      Begin VB.CheckBox chkPeriod 
         Caption         =   ". (Period)"
         Height          =   255
         Left            =   1680
         TabIndex        =   13
         Top             =   1440
         Value           =   1  'Checked
         Width           =   1335
      End
      Begin VB.CheckBox chkMinus 
         Caption         =   "- (Minus)"
         Height          =   255
         Left            =   1680
         TabIndex        =   12
         Top             =   1200
         Value           =   1  'Checked
         Width           =   1335
      End
      Begin VB.CheckBox chkPlus 
         Caption         =   "+"
         Height          =   255
         Left            =   1680
         TabIndex        =   10
         Top             =   720
         Value           =   1  'Checked
         Width           =   1335
      End
      Begin VB.CheckBox chkAsterisk 
         Caption         =   "*"
         Height          =   255
         Left            =   1680
         TabIndex        =   9
         Top             =   480
         Value           =   1  'Checked
         Width           =   1335
      End
      Begin VB.CheckBox chkApostrophe 
         Caption         =   "' (Apostrophe)"
         Height          =   255
         Left            =   240
         TabIndex        =   6
         Top             =   1680
         Value           =   1  'Checked
         Width           =   1335
      End
      Begin VB.CheckBox chkAmpersand 
         Caption         =   "&&"
         Height          =   255
         Left            =   240
         TabIndex        =   5
         Top             =   1440
         Value           =   1  'Checked
         Width           =   1335
      End
      Begin VB.CheckBox chkExclamation 
         Caption         =   "!"
         Height          =   255
         Left            =   240
         TabIndex        =   0
         Top             =   240
         Value           =   1  'Checked
         Width           =   1335
      End
   End
End
Attribute VB_Name = "frmSpecial"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = False
Attribute VB_PredeclaredId = True
Attribute VB_Exposed = False
Private Sub chkAmpersand_Click()
  frmPassGen.useAmpersand = chkAmpersand.Value
End Sub

Private Sub chkApostrophe_Click()
   frmPassGen.useApostrophe = chkApostrophe.Value
End Sub

Private Sub chkAsterisk_Click()
  frmPassGen.useAsterisk = chkAsterisk.Value
End Sub

Private Sub chkAtSign_Click()
  frmPassGen.useAtSign = chkAtSign.Value
End Sub

Private Sub chkBackSlash_Click()
  frmPassGen.useBackSlash = chkBackSlash.Value
End Sub

Private Sub chkColon_Click()
  frmPassGen.useColon = chkColon.Value
End Sub

Private Sub chkComma_Click()
  frmPassGen.useComma = chkComma.Value
End Sub

Private Sub chkDollar_Click()
  frmPassGen.useDollar = chkDollar.Value
End Sub

Private Sub chkEquals_Click()
  frmPassGen.useEquals = chkEquals.Value
End Sub

Private Sub chkExclamation_Click()
  frmPassGen.useExclamation = chkExclamation.Value
End Sub

Private Sub chkForwardSlash_Click()
  frmPassGen.useForwardSlash = chkForwardSlash.Value
End Sub

Private Sub chkGrave_Click()
  frmPassGen.useGrave = chkGrave.Value
End Sub

Private Sub chkGreaterThan_Click()
  frmPassGen.useGreaterThan = chkGreaterThan.Value
End Sub

Private Sub chkHash_Click()
  frmPassGen.useHash = chkHash.Value
End Sub

Private Sub chkLeftBrace_Click()
  frmPassGen.useLeftBrace = chkLeftBrace.Value
End Sub

Private Sub chkLeftBracket_Click()
  frmPassGen.useLeftBracket = chkLeftBracket.Value
End Sub

Private Sub chkLeftParenthesis_Click()
  frmPassGen.useLeftParenthesis = chkLeftParenthesis.Value
End Sub

Private Sub chkLessThan_Click()
  frmPassGen.useLessThan = chkLessThan.Value
End Sub

Private Sub chkMinus_Click()
  frmPassGen.useMinus = chkMinus.Value
End Sub

Private Sub chkPercent_Click()
  frmPassGen.usePercent = chkPercent.Value
End Sub

Private Sub chkPeriod_Click()
  frmPassGen.usePeriod = chkPeriod.Value
End Sub

Private Sub chkPipe_Click()
  frmPassGen.usePipe = chkPipe.Value
End Sub

Private Sub chkPlus_Click()
  frmPassGen.usePlus = chkPlus.Value
End Sub

Private Sub chkPower_Click()
  frmPassGen.usePower = chkPower.Value
End Sub

Private Sub chkQuestion_Click()
  frmPassGen.useQuestion = chkQuestion.Value
End Sub

Private Sub chkQuote_Click()
  frmPassGen.useQuote = chkQuote.Value
End Sub

Private Sub chkRightBrace_Click()
  frmPassGen.useRightBrace = chkRightBrace.Value
End Sub

Private Sub chkRightBracket_Click()
  frmPassGen.useRightBracket = chkRightBracket.Value
End Sub

Private Sub chkRightParenthesis_Click()
  frmPassGen.useRightParenthesis = chkRightParenthesis.Value
End Sub

Private Sub chkSemiColon_Click()
  frmPassGen.useSemiColon = chkSemiColon.Value
End Sub

Private Sub chkTilde_Click()
  frmPassGen.useTilde = chkTilde.Value
End Sub

Private Sub chkUnderscore_Click()
  frmPassGen.useUnderscore = chkUnderscore.Value
End Sub

Private Sub cmdExclude_Click()
  If chkExclamation.Value = 1 Then chkExclamation.Value = 0
  If chkQuote.Value = 1 Then chkQuote.Value = 0
  If chkHash.Value = 1 Then chkHash.Value = 0
  If chkDollar.Value = 1 Then chkDollar.Value = 0
  If chkPercent.Value = 1 Then chkPercent.Value = 0
  If chkAmpersand.Value = 1 Then chkAmpersand.Value = 0
  If chkApostrophe.Value = 1 Then chkApostrophe.Value = 0
  If chkLeftParenthesis.Value = 1 Then chkLeftParenthesis.Value = 0
  If chkRightParenthesis.Value = 1 Then chkRightParenthesis.Value = 0
  If chkAsterisk.Value = 1 Then chkAsterisk.Value = 0
  If chkPlus.Value = 1 Then chkPlus.Value = 0
  If chkComma.Value = 1 Then chkComma.Value = 0
  If chkMinus.Value = 1 Then chkMinus.Value = 0
  If chkPeriod.Value = 1 Then chkPeriod.Value = 0
  If chkForwardSlash.Value = 1 Then chkForwardSlash.Value = 0
  If chkColon.Value = 1 Then chkColon.Value = 0
  If chkSemiColon.Value = 1 Then chkSemiColon.Value = 0
  If chkLessThan.Value = 1 Then chkLessThan.Value = 0
  If chkEquals.Value = 1 Then chkEquals.Value = 0
  If chkGreaterThan.Value = 1 Then chkGreaterThan.Value = 0
  If chkQuestion.Value = 1 Then chkQuestion.Value = 0
  If chkAtSign.Value = 1 Then chkAtSign.Value = 0
  If chkLeftBracket.Value = 1 Then chkLeftBracket.Value = 0
  If chkBackSlash.Value = 1 Then chkBackSlash.Value = 0
  If chkRightBracket.Value = 1 Then chkRightBracket.Value = 0
  If chkPower.Value = 1 Then chkPower.Value = 0
  If chkUnderscore.Value = 1 Then chkUnderscore.Value = 0
  If chkGrave.Value = 1 Then chkGrave.Value = 0
  If chkLeftBrace.Value = 1 Then chkLeftBrace.Value = 0
  If chkPipe.Value = 1 Then chkPipe.Value = 0
  If chkRightBrace.Value = 1 Then chkRightBrace.Value = 0
  If chkTilde.Value = 1 Then chkTilde.Value = 0
  ' `-> ASCII table order.
End Sub

Private Sub cmdIncludeAll_Click()
  If chkExclamation.Value = 0 Then chkExclamation.Value = 1
  If chkQuote.Value = 0 Then chkQuote.Value = 1
  If chkHash.Value = 0 Then chkHash.Value = 1
  If chkDollar.Value = 0 Then chkDollar.Value = 1
  If chkPercent.Value = 0 Then chkPercent.Value = 1
  If chkAmpersand.Value = 0 Then chkAmpersand.Value = 1
  If chkApostrophe.Value = 0 Then chkApostrophe.Value = 1
  If chkLeftParenthesis.Value = 0 Then chkLeftParenthesis.Value = 1
  If chkRightParenthesis.Value = 0 Then chkRightParenthesis.Value = 1
  If chkAsterisk.Value = 0 Then chkAsterisk.Value = 1
  If chkPlus.Value = 0 Then chkPlus.Value = 1
  If chkComma.Value = 0 Then chkComma.Value = 1
  If chkMinus.Value = 0 Then chkMinus.Value = 1
  If chkPeriod.Value = 0 Then chkPeriod.Value = 1
  If chkForwardSlash.Value = 0 Then chkForwardSlash.Value = 1
  If chkColon.Value = 0 Then chkColon.Value = 1
  If chkSemiColon.Value = 0 Then chkSemiColon.Value = 1
  If chkLessThan.Value = 0 Then chkLessThan.Value = 1
  If chkEquals.Value = 0 Then chkEquals.Value = 1
  If chkGreaterThan.Value = 0 Then chkGreaterThan.Value = 1
  If chkQuestion.Value = 0 Then chkQuestion.Value = 1
  If chkAtSign.Value = 0 Then chkAtSign.Value = 1
  If chkLeftBracket.Value = 0 Then chkLeftBracket.Value = 1
  If chkBackSlash.Value = 0 Then chkBackSlash.Value = 1
  If chkRightBracket.Value = 0 Then chkRightBracket.Value = 1
  If chkPower.Value = 0 Then chkPower.Value = 1
  If chkUnderscore.Value = 0 Then chkUnderscore.Value = 1
  If chkGrave.Value = 0 Then chkGrave.Value = 1
  If chkLeftBrace.Value = 0 Then chkLeftBrace.Value = 1
  If chkPipe.Value = 0 Then chkPipe.Value = 1
  If chkRightBrace.Value = 0 Then chkRightBrace.Value = 1
  If chkTilde.Value = 0 Then chkTilde.Value = 1
  ' `-> ASCII table order.
End Sub

Private Sub cmdOkay_Click()
  Unload Me
End Sub

Private Sub Form_KeyPress(KeyAscii As Integer)
  If KeyAscii = vbKeyEscape Then Unload Me
End Sub

Private Sub Form_Load()
  chkExclamation.Value = IIf(frmPassGen.useExclamation = 1, 1, 0)
  chkQuote.Value = IIf(frmPassGen.useQuote = 1, 1, 0)
  chkHash.Value = IIf(frmPassGen.useHash = 1, 1, 0)
  chkDollar.Value = IIf(frmPassGen.useDollar = 1, 1, 0)
  chkPercent.Value = IIf(frmPassGen.usePercent = 1, 1, 0)
  chkAmpersand.Value = IIf(frmPassGen.useAmpersand = 1, 1, 0)
  chkApostrophe.Value = IIf(frmPassGen.useApostrophe = 1, 1, 0)
  chkLeftParenthesis.Value = IIf(frmPassGen.useLeftParenthesis = 1, 1, 0)
  chkRightParenthesis.Value = IIf(frmPassGen.useRightParenthesis = 1, 1, 0)
  chkAsterisk.Value = IIf(frmPassGen.useAsterisk = 1, 1, 0)
  chkPlus.Value = IIf(frmPassGen.usePlus = 1, 1, 0)
  chkComma.Value = IIf(frmPassGen.useComma = 1, 1, 0)
  chkMinus.Value = IIf(frmPassGen.useMinus = 1, 1, 0)
  chkPeriod.Value = IIf(frmPassGen.usePeriod = 1, 1, 0)
  chkForwardSlash.Value = IIf(frmPassGen.useForwardSlash = 1, 1, 0)
  chkColon.Value = IIf(frmPassGen.useColon = 1, 1, 0)
  chkSemiColon.Value = IIf(frmPassGen.useSemiColon = 1, 1, 0)
  chkLessThan.Value = IIf(frmPassGen.useLessThan = 1, 1, 0)
  chkEquals.Value = IIf(frmPassGen.useEquals = 1, 1, 0)
  chkGreaterThan.Value = IIf(frmPassGen.useGreaterThan = 1, 1, 0)
  chkQuestion.Value = IIf(frmPassGen.useQuestion = 1, 1, 0)
  chkAtSign.Value = IIf(frmPassGen.useAtSign = 1, 1, 0)
  chkLeftBracket.Value = IIf(frmPassGen.useLeftBracket = 1, 1, 0)
  chkBackSlash.Value = IIf(frmPassGen.useBackSlash = 1, 1, 0)
  chkRightBracket.Value = IIf(frmPassGen.useRightBracket = 1, 1, 0)
  chkPower.Value = IIf(frmPassGen.usePower = 1, 1, 0)
  chkUnderscore.Value = IIf(frmPassGen.useUnderscore = 1, 1, 0)
  chkGrave.Value = IIf(frmPassGen.useGrave = 1, 1, 0)
  chkLeftBrace.Value = IIf(frmPassGen.useLeftBrace = 1, 1, 0)
  chkPipe.Value = IIf(frmPassGen.usePipe = 1, 1, 0)
  chkRightBrace.Value = IIf(frmPassGen.useRightBrace = 1, 1, 0)
  chkTilde.Value = IIf(frmPassGen.useTilde = 1, 1, 0)
  ' `-> ASCII table order.
End Sub

Private Sub Form_Terminate()
  End
  ' `-> I doubt this has any use; but just incase...
End Sub

Private Sub Form_Unload(Cancel As Integer)
  frmPassGen.makeSpecialString
End Sub

' EOF
