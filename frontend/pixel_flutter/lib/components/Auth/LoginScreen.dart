//import 'package:flutter/material.dart';
//import 'package:pixel_flutter/blocs/RegisterBlocs.dart';
//
//class LoginScreen extends StatelessWidget {
//
//  @override
//  Widget build(BuildContext context) {
//    return Container(
//      margin: EdgeInsets.all(20.0),
//      child: Column(
//        children: <Widget>[
//          emailField(_inputBloc),
//          usernameField(_inputBloc),
//          passwordField(_inputBloc),
//          Container(
//            margin: EdgeInsets.only(top: 25.0),
//          ),
//          submitButton(_inputBloc),
//        ],
//      ),
//    );
//  }
//
//  Widget emailField(InputBloc _inputBloc) {
//    return StreamBuilder(
//      stream: _inputBloc.email,
//      builder: (context, snapshot) {
//        return TextField(
//          onChanged: _inputBloc.changeEmail,
//          keyboardType: TextInputType.emailAddress,
//          decoration: InputDecoration(
//            hintText: 'ypu@example.com',
//            labelText: 'Email Address',
//            errorText: snapshot.error,
//          ),
//        );
//      },
//    );
//  }
//
//  Widget passwordField(InputBloc _inputBloc) {
//    return StreamBuilder(
//        stream: _inputBloc.password,
//        builder: (context, snapshot) {
//          return TextField(
//            obscureText: true,
//            onChanged: _inputBloc.changePassword,
//            decoration: InputDecoration(
//              hintText: 'Password',
//              labelText: 'Password',
//              errorText: snapshot.error,
//            ),
//          );
//        });
//  }
//
//  Widget usernameField(InputBloc _inputBloc) {
//    return StreamBuilder(
//        stream: _inputBloc.username,
//        builder: (context, snapshot) {
//          return TextField(
//            keyboardType: TextInputType.text,
//            onChanged: _inputBloc.changeUsername,
//            decoration: InputDecoration(
//              hintText: 'JohnDoe',
//              labelText: 'Username',
//              errorText: snapshot.error,
//            ),
//          );
//        });
//  }
//
//  Widget submitButton(InputBloc _inputBloc) {
//    return StreamBuilder(
//      stream: _inputBloc.registerValid,
//      builder: (context, snapshot) {
//        return RaisedButton(
//          child: Text('Login'),
//          color: Colors.blue,
//          onPressed: snapshot.hasData ? _inputBloc.registerValid : null,
//        );
//      },
//    );
//  }
//}
