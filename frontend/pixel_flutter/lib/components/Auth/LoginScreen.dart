import 'package:flutter/material.dart';
import 'package:pixel_flutter/blocs/AuthBloc/AuthBloc.dart';
import 'package:pixel_flutter/blocs/Provider.dart';

class LoginScreen extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    final bloc = Provider.of(context);

    return Container(
      margin: EdgeInsets.all(20.0),
      child: Column(
        children: <Widget>[
          emailField(bloc),
          usernameField(bloc),
          passwordField(bloc),
          Container(
            margin: EdgeInsets.only(top: 25.0),
          ),
          submitButton(bloc),
        ],
      ),
    );
  }

  Widget emailField(AuthBloc bloc) {
    return StreamBuilder(
      stream: bloc.email,
      builder: (context, snapshot) {
        return TextField(
          onChanged: bloc.changeEmail,
          keyboardType: TextInputType.emailAddress,
          decoration: InputDecoration(
            hintText: 'ypu@example.com',
            labelText: 'Email Address',
            errorText: snapshot.error,
          ),
        );
      },
    );
  }

  Widget passwordField(AuthBloc bloc) {
    return StreamBuilder(
        stream: bloc.password,
        builder: (context, snapshot) {
          return TextField(
            obscureText: true,
            onChanged: bloc.changePassword,
            decoration: InputDecoration(
              hintText: 'Password',
              labelText: 'Password',
              errorText: snapshot.error,
            ),
          );
        });
  }

  Widget usernameField(AuthBloc bloc) {
    return StreamBuilder(
        stream: bloc.username,
        builder: (context, snapshot) {
          return TextField(
            keyboardType: TextInputType.text,
            onChanged: bloc.changeUsername,
            decoration: InputDecoration(
              hintText: 'JohnDoe',
              labelText: 'Username',
              errorText: snapshot.error,
            ),
          );
        });
  }

  Widget submitButton(AuthBloc bloc) {
    return StreamBuilder(
      stream: bloc.registerValid,
      builder: (context, snapshot) {
        return RaisedButton(
          child: Text('Login'),
          color: Colors.blue,
          onPressed: snapshot.hasData ? bloc.submit : null,
        );
      },
    );
  }
}
