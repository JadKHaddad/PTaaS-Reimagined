import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:models/models.dart';

void main() {
  String generalResponse =
      '{"success":false,"data":null,"error":{"code":"500","message":"Internal Server Error"}}';
  GeneralResponse response = GeneralResponse.deserialize(
      BincodeDeserializer(Uint8List.fromList(utf8.encode(generalResponse))));
  print(response);
  runApp(const MainApp());
}

class BincodeDeserializer extends BinaryDeserializer {
  BincodeDeserializer(Uint8List input) : super(input);

  @override
  int deserializeLength() {
    // bincode sends this as a u64 but since transferred data length should never exceed the upper
    // bounds of an i64 (9223372036854775807 bytes is 9k petabytes) still deserialize to a Dart int
    return deserializeInt64();
  }

  @override
  int deserializeVariantIndex() {
    return deserializeUint32();
  }

  @override
  void checkThatKeySlicesAreIncreasing(Slice key1, Slice key2) {
    // Not required by the format.
  }
}

class MainApp extends StatelessWidget {
  const MainApp({super.key});

  @override
  Widget build(BuildContext context) {
    return const MaterialApp(
      home: Scaffold(
        body: Center(
          child: Text('Hello World!'),
        ),
      ),
    );
  }
}
