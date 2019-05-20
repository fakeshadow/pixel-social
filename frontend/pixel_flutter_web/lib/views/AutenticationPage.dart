import 'package:flutter_web/material.dart';

import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_web/widgets.dart';
import 'package:pixel_flutter_web/blocs/ErrorBlocs.dart';

import 'package:pixel_flutter_web/blocs/RegisterBlocs.dart';
import 'package:pixel_flutter_web/blocs/UserBlocs.dart';
import 'package:pixel_flutter_web/components/GeneralBackground.dart';
import 'package:pixel_flutter_web/components/AnimatedSubmitButton.dart';
import 'package:pixel_flutter_web/components/AuthenticationNavBar.dart';
import 'package:pixel_flutter_web/style/text.dart';

/// pass in type and username for login form and type only for register
class AuthenticationPage extends StatefulWidget {
  final String type, username;

  AuthenticationPage({@required this.type, this.username});

  @override
  _AuthenticationPageState createState() => _AuthenticationPageState();
}

class _AuthenticationPageState extends State<AuthenticationPage>
    with SingleTickerProviderStateMixin {
  RegisterBloc _registerBloc;
  UserBloc _userBloc;
  String _type;

  AnimationController _animationController;
  Animation<double> _animationDouble;

  final TextEditingController emailController = TextEditingController();
  final TextEditingController usernameController = TextEditingController();
  final TextEditingController passwordController = TextEditingController();

  initAnimation() {
    _animationController.forward();
  }

  @override
  void initState() {
    _type = widget.type;
    if (_type == 'Login') {
      usernameController.text = widget.username;
    }
    usernameController.text = widget.username;

    _registerBloc = RegisterBloc();
    _userBloc = BlocProvider.of<UserBloc>(context);
    usernameController.addListener(_onUsernameChanged);
    emailController.addListener(_onEmailChanged);
    passwordController.addListener(_onPasswordChanged);

    _animationController =
        AnimationController(vsync: this, duration: Duration(milliseconds: 250));
    _animationDouble =
        Tween<double>(begin: 0.0, end: 1.0).animate(_animationController);

    super.initState();
  }

  @override
  Widget build(BuildContext context) {
    return BlocBuilder(
        bloc: _registerBloc,
        builder: (BuildContext context, RegisterState state) {
          _registerBloc
              .dispatch(UsernameChanged(username: usernameController.text));
          return Hero(
              tag: 'auth',
              child: Scaffold(
                body: BlocListener(
                    //ToDo: Change error handling to error bloc
                    bloc: _userBloc,
                    listener: (context, userState) async {
                      _snackController(context, userState);
                    },
                    child: _authStack(state)),
              ));
        });
  }

  Widget _authStack(RegisterState state) {
    return Stack(children: <Widget>[
      GeneralBackground(),
      FutureBuilder(
          future: initAnimation(),
          builder: (context, snapshot) => ScaleTransition(
              scale: _animationDouble,
              child: SingleChildScrollView(
                  child: Center(
                child: SizedBox(
                  width: 500,
                  child: Column(
                    mainAxisSize: MainAxisSize.max,
                    children: <Widget>[
                      AuthNavBar(),
                      Material(
                          color: Colors.transparent,
                          child: Text('PixelShare', style: logoStyle)),
                      _inputForm(state),
                      SizedBox(
                        height: 20,
                      ),
                      SubmitAnimatedButton(
                          state: state,
                          type: _type,
                          submit: () => _submit(state)),
                      _flatButtonChoice(_type, state)
                    ],
                  ),
                ),
              ))))
    ]);
  }

  Widget _inputForm(state) {
    return Form(
        child: ListView(shrinkWrap: true, children: <Widget>[
      _usernameField(state),
      _type == 'Register' || _type == 'Recover'
          ? _emailField(state)
          : Container(),
      _type != 'Recover' ? _passwordField(state) : Container(),
    ]));
  }

  Widget _flatButtonChoice(String type, state) {
    if (type == 'Register') {
      return _flatButton(
          text: 'Already have account?',
          function: () => _changeAuthType(type: 'Login'));
    } else if (type == 'Login') {
      return _flatButton(
          text: 'Forgot Password?',
          function: () => _changeAuthType(type: 'Recover'));
    } else {
      return Container();
    }
  }

  Widget _usernameField(RegisterState state) {
    return Padding(
      padding: EdgeInsets.symmetric(horizontal: 5),
      child: Material(
        borderRadius: BorderRadius.circular(10.0),
        color: Colors.white.withOpacity(0.1),
        elevation: 0,
        child: TextFormField(
          controller: usernameController,
          decoration: InputDecoration(
            icon: Icon(Icons.person_outline),
            labelText: 'Username',
          ),
          autovalidate: true,
          validator: (_) {
            return state.isUsernameValid || state.username.isEmpty
                ? null
                : 'Invalid Username';
          },
        ),
      ),
    );
  }

  Widget _emailField(RegisterState state) {
    return Padding(
        padding: EdgeInsets.symmetric(horizontal: 5),
        child: Material(
            borderRadius: BorderRadius.circular(10.0),
            color: Colors.white.withOpacity(0.1),
            elevation: 0,
            child: TextFormField(
              controller: emailController,
              decoration: InputDecoration(
                icon: Icon(Icons.email),
                labelText: 'Email',
              ),
              autovalidate: true,
              validator: (_) {
                return state.isEmailValid || state.email.isEmpty
                    ? null
                    : 'Invalid Email';
              },
            )));
  }

  Widget _passwordField(RegisterState state) {
    return Padding(
      padding: EdgeInsets.symmetric(horizontal: 5),
      child: Material(
        borderRadius: BorderRadius.circular(10.0),
        color: Colors.white.withOpacity(0.1),
        elevation: 0,
        child: TextFormField(
          controller: passwordController,
          decoration: InputDecoration(
            icon: Icon(Icons.lock_outline),
            labelText: 'Password',
          ),
          obscureText: true,
          autovalidate: true,
          validator: (_) {
            return state.isPasswordValid || state.password.isEmpty
                ? null
                : 'Invalid Password';
          },
        ),
      ),
    );
  }

  Widget _flatButton({String text, Function function}) {
    return Padding(
      padding: EdgeInsets.symmetric(horizontal: 70, vertical: 0),
      child: FlatButton(
          color: Colors.transparent,
          onPressed: function,
          child: Text(text, style: recoverButtonStyle)),
    );
  }

  void _snackController(context, userState) async {
    if (userState is UserLoaded) {
      BlocProvider.of<ErrorBloc>(context)
          .dispatch(GetSuccess(success: 'Login Success'));
      Navigator.pop(context);
    }
    if (userState is Failure) {
      Scaffold.of(context).showSnackBar(SnackBar(
        duration: Duration(seconds: 2),
        content: Text(userState.error),
        backgroundColor: Colors.deepOrange,
      ));
    }
  }

  void _changeAuthType({String type}) {
    setState(() {
      _type = type;
      _animationController.reset();
      _animationController.forward();
    });
  }

  @override
  void dispose() {
    usernameController.dispose();
    emailController.dispose();
    passwordController.dispose();
    _registerBloc.dispose();
    _animationController.dispose();
    super.dispose();
  }

  void _onUsernameChanged() {
    _registerBloc.dispatch(UsernameChanged(username: usernameController.text));
  }

  void _onEmailChanged() {
    _registerBloc.dispatch(EmailChanged(email: emailController.text));
  }

  void _onPasswordChanged() {
    _registerBloc.dispatch(PasswordChanged(password: passwordController.text));
  }

  void _submit(RegisterState state) {
    if (_type == 'Register') {
      _userBloc.dispatch(Registering(
          username: state.username,
          password: state.password,
          email: state.email));
    } else if (_type == 'Login') {
      _userBloc.dispatch(
          LoggingIn(username: state.username, password: state.password));
    } else if (_type == 'Recover') {
      print('recover');
    }
  }
}
