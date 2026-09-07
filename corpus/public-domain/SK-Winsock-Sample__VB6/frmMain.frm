VERSION 5.00
Object = "{248DD890-BB45-11CF-9ABC-0080C7E7B78D}#1.0#0"; "MSWINSCK.OCX"
Begin VB.Form frmMain 
   BorderStyle     =   3  'Fixed Dialog
   Caption         =   "Connect to server"
   ClientHeight    =   4425
   ClientLeft      =   45
   ClientTop       =   330
   ClientWidth     =   4830
   Icon            =   "frmMain.frx":0000
   LinkTopic       =   "Form1"
   LockControls    =   -1  'True
   MaxButton       =   0   'False
   MinButton       =   0   'False
   ScaleHeight     =   4425
   ScaleWidth      =   4830
   ShowInTaskbar   =   0   'False
   StartUpPosition =   3  'Windows Default
   Begin VB.Frame Frame2 
      Height          =   1095
      Left            =   60
      TabIndex        =   5
      Top             =   3240
      Width           =   4710
      Begin VB.CommandButton cmdSend 
         Caption         =   "&Send"
         Enabled         =   0   'False
         Height          =   345
         Left            =   3180
         TabIndex        =   7
         Top             =   600
         Width           =   1395
      End
      Begin VB.TextBox txtSend 
         Enabled         =   0   'False
         Height          =   345
         Left            =   120
         TabIndex        =   6
         Text            =   "GET index.html"
         ToolTipText     =   "Data to Send"
         Top             =   210
         Width           =   4455
      End
   End
   Begin VB.Frame Frame1 
      Height          =   3225
      Left            =   60
      TabIndex        =   0
      Top             =   0
      Width           =   4710
      Begin VB.TextBox txtServer 
         Height          =   345
         Left            =   120
         TabIndex        =   4
         Text            =   "www.yahoo.com"
         ToolTipText     =   "Server"
         Top             =   210
         Width           =   2385
      End
      Begin VB.TextBox txtPort 
         Height          =   345
         Left            =   2550
         TabIndex        =   3
         Text            =   "80"
         ToolTipText     =   "Port"
         Top             =   210
         Width           =   615
      End
      Begin VB.CommandButton cmdConnect 
         Caption         =   "&Connect!"
         Default         =   -1  'True
         Height          =   345
         Left            =   3210
         TabIndex        =   2
         Top             =   210
         Width           =   1365
      End
      Begin VB.TextBox txtReceived 
         Height          =   2505
         Left            =   120
         Locked          =   -1  'True
         MultiLine       =   -1  'True
         ScrollBars      =   2  'Vertical
         TabIndex        =   1
         ToolTipText     =   "Received Data"
         Top             =   600
         Width           =   4455
      End
      Begin MSWinsockLib.Winsock wsPop 
         Left            =   840
         Top             =   780
         _ExtentX        =   741
         _ExtentY        =   741
         _Version        =   393216
      End
   End
End
Attribute VB_Name = "frmMain"
Attribute VB_GlobalNameSpace = False
Attribute VB_Creatable = False
Attribute VB_PredeclaredId = True
Attribute VB_Exposed = False
Option Explicit
Dim blnConnected As Boolean

Private Sub cmdConnect_Click()
  '/* Close the Winsock, just to make sure...
  '*/
  wsPop.Close
  Do
    DoEvents
  Loop While wsPop.State <> sckClosed
  
  '/* Connect to the specified server and port...
  '*/
  wsPop.Connect txtServer.Text, txtPort.Text
  
End Sub

Private Sub cmdSend_Click()
  '/* Send the data to the server, and clear the textbox...
  '*/
  wsPop.SendData txtSend & vbCrLf
  txtSend.Text = ""
  txtSend.SetFocus
End Sub



'/* This event is triggered as soon as a connection is established...
'*/
Private Sub wsPop_Connect()
  blnConnected = True
  txtSend.Enabled = True
  cmdSend.Enabled = True
  txtReceived.Text = "Connected to " & wsPop.RemoteHost
End Sub
'/* This event is triggered as soon as the connection is terminated by the server...
'*/
Private Sub wsPop_Close()
  blnConnected = False
  txtSend.Enabled = False
  cmdSend.Enabled = False
End Sub

'/* This event is triggered as soon as data is arriving from the server...
'*/
Private Sub wsPop_DataArrival(ByVal bytesTotal As Long)
Dim strData As String

  '/* Get the data and display it...
  '*/
  wsPop.GetData strData
  txtReceived.Text = txtReceived.Text & vbNewLine & strData
  txtReceived.SelStart = Len(txtReceived.Text)
  
End Sub
