import 'dart:convert';
import 'package:flutter/material.dart';
import 'models_2.dart';
import 'msg.dart';

void main() {
  runApp(const MainApp());
}

class MainApp extends StatelessWidget {
  const MainApp({super.key});

  void _doSomeAPIStuff() {
    String apiFailed = '{"failed":"missingToken"}';
    String allProj =
        '{"processed":{"allProjects":{"processed":{"projects":[{"id":"id","installed":true,"scripts":[{"id":"id"}]}]}}}}';
    String allProjFailed =
        '{"processed":{"allProjects":{"failed":"aProjectIsMissing"}}}';
    String allScripts =
        '{"processed":{"allScripts":{"processed":{"scripts":[{"id":"id"}]}}}}';
    String allScriptsFailed =
        '{"processed":{"allScripts":{"failed":"aScriptIsMissing"}}}';

    Map<String, dynamic> jsonApiFailed = jsonDecode(apiFailed);

    Map<String, dynamic> jsonAllProj = jsonDecode(allProj);
    Map<String, dynamic> jsonAllProjFailed = jsonDecode(allProjFailed);

    Map<String, dynamic> jsonAllScripts = jsonDecode(allScripts);
    Map<String, dynamic> jsonAllScriptsFailed = jsonDecode(allScriptsFailed);

    APIResponse apiResponse = APIResponse.fromJson(jsonApiFailed);
    if (apiResponse.processed != null) {
      print("API processed: \n");
      if (apiResponse.processed!.allProjects != null) {
        print("All Projects: ");
        if (apiResponse.processed!.allProjects!.processed != null) {
          print(
              "All Projects Procced: ${apiResponse.processed!.allProjects!.processed}");
        } else if (apiResponse.processed!.allProjects!.failed != null) {
          print(
              "All Projects Failed: ${apiResponse.processed!.allProjects!.failed}");
        }
      } else if (apiResponse.processed!.allScripts != null) {
        print("All Scripts: ");
        if (apiResponse.processed!.allScripts!.processed != null) {
          print(
              "All Scripts Procced: ${apiResponse.processed!.allScripts!.processed}");
        } else if (apiResponse.processed!.allScripts!.failed != null) {
          print(
              "All Scripts Failed: ${apiResponse.processed!.allScripts!.failed}");
        }
      }
    } else if (apiResponse.failed != null) {
      print("API failed: ${apiResponse.failed}");
    }
  }

  void _doWSStuff() {
    String wsMessage = '{"Subscribe":{"project_id":"project1"}}';
    // we already know the type of the message 'WSFromClient', so we can just parse it as such
    Map<String, dynamic> json = jsonDecode(wsMessage);
    WSFromClient fromClient = WSFromClient.fromJson(json);
    print("WS message: $fromClient");
    print("sub: ${fromClient.subscribe}, unsub: ${fromClient.unsubscribe}");
    // is this a subscribe message? or something else?
    if (fromClient.subscribe != null) {
      print("Subscribe message: ${fromClient.subscribe}");
    }

    // is this a unsubscribe message? or something else?
    if (fromClient.unsubscribe != null) {
      print("Unsubscribe message: ${fromClient.unsubscribe}");
    }
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      home: Scaffold(
        body: Center(
            child: Column(
          children: [
            TextButton(
                onPressed: _doSomeAPIStuff,
                child: const Text("Do some API stuff")),
            TextButton(onPressed: _doWSStuff, child: const Text("Do WS stuff")),
          ],
        )),
      ),
    );
  }
}
