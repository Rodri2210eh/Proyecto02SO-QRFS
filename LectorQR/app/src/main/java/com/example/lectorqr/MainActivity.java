package com.example.lectorqr;

import android.annotation.SuppressLint;
import android.content.Intent;
import android.os.Bundle;
import android.view.View;
import android.widget.Button;
import android.widget.EditText;
import android.widget.Toast;

import androidx.appcompat.app.AppCompatActivity;

import com.google.zxing.integration.android.IntentIntegrator;
import com.google.zxing.integration.android.IntentResult;


public class MainActivity extends AppCompatActivity {

    //se establecen los botones y textos para poder trabajar con ellos
    Button btnScan;
    Button btnSendEmail;
    EditText txtResultado;
    EditText txtEmail;
    EditText txtSubject;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_main);

        //se definen los botones y mensajes usando las vistas
        btnScan = findViewById(R.id.btnScan);
        txtResultado = findViewById(R.id.txtResultado);
        btnSendEmail = findViewById(R.id.btnSendEmail);
        txtEmail = findViewById(R.id.txtEmail);
        txtSubject = findViewById(R.id.txtSubject);

        //Al oprimir el boton de scaneo se ejecuta lo siguiente:
        btnScan.setOnClickListener(view -> {

            IntentIntegrator integrador = new IntentIntegrator(MainActivity.this);
            integrador.setDesiredBarcodeFormats(IntentIntegrator.ALL_CODE_TYPES);
            integrador.setPrompt("Lector de QR - SO TEC");
            integrador.setCameraId(1); //camara trasera
            integrador.setBeepEnabled(true);
            integrador.setBarcodeImageEnabled(true);
            integrador.initiateScan();

        });

        //Al oprimir el boton de enviar correo se llama la funcion sendEmail()
        btnSendEmail.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                sendMail();
            }
        });
    }

    //Se verifica se se cancela la lectura del codigo qr o si se obtiene un nulo
    @SuppressLint("QueryPermissionsNeeded")
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        IntentResult result = IntentIntegrator.parseActivityResult(requestCode, resultCode, data);

        if(result != null) {
            if(result.getContents() == null) {
                Toast.makeText(this, "Se ha cancelado la lectura del QR", Toast.LENGTH_LONG).show();
            } else {
                Toast.makeText(this,result.getContents(), Toast.LENGTH_LONG).show();
                txtResultado.setText(result.getContents());

            }
        } else {
            super.onActivityResult(requestCode, resultCode, data);
        }

    }

    //funcion que hace posible el envio del email usando la informacion obtenida de la lectura qr
    private void sendMail() {
        String recipientList = txtEmail.getText().toString();
        String[] recipients = recipientList.split(",");

        String subject = txtSubject.getText().toString();
        String message = txtResultado.getText().toString();

        Intent intent = new Intent(Intent.ACTION_SEND);
        intent.putExtra(Intent.EXTRA_EMAIL, recipients);
        intent.putExtra(Intent.EXTRA_SUBJECT, subject);
        intent.putExtra(Intent.EXTRA_TEXT, message);

        if(!txtResultado.getText().toString().isEmpty()) {
            intent.setType("message/rfc822");
            startActivity(Intent.createChooser(intent, "Seleccione la app para el envío"));
        } else {
            Toast.makeText(this, "Asegurese de tener un resultado de lectura QR primero", Toast.LENGTH_LONG).show();
        }
    }
}