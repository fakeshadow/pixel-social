import 'package:flutter_web/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:pixel_flutter_web/blocs/ErrorBlocs.dart';

void main() => runApp(MyApp());

class MyApp extends StatelessWidget {
  final ErrorBloc errorBloc = ErrorBloc();

  @override
  Widget build(BuildContext context) {
    return BlocProvider(
      bloc: errorBloc,
      child: MaterialApp(
        title: 'Pixel Flutter Web',
        theme: ThemeData(
          primarySwatch: Colors.blue,
        ),
        home: HomePage(title: 'PixelShare'),
      ),
    );
  }
}

class HomePage extends StatelessWidget {
  HomePage({Key key, this.title}) : super(key: key);

  final String title;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: BlocListener(
        bloc: BlocProvider.of<ErrorBloc>(context),
        listener: (BuildContext context, ErrorState state) async {
          if (state is NoSnack) {
            Scaffold.of(context).hideCurrentSnackBar();
          } else if (state is ShowSuccess) {
            Scaffold.of(context).showSnackBar(SnackBar(
              duration: Duration(seconds: 2),
              backgroundColor: Colors.green,
              content: Text(state.success),
            ));
          } else if (state is ShowError) {
            Scaffold.of(context).showSnackBar(SnackBar(
              duration: Duration(seconds: 2),
              backgroundColor: Colors.deepOrangeAccent,
              content: Text(state.error),
            ));
          }
        },
        child: CustomScrollView(
          slivers: <Widget>[
            SliverAppBar(
              floating: true,
              snap: true,
              title: Text("Pixel fultter web test"),
              leading: IconButton(
                onPressed: () { BlocProvider.of<ErrorBloc>(context).dispatch(GetSuccess(success: "You pressed something"));},
                icon: Icon(Icons.apps),
              ),
            ),
            SliverFillViewport(
              delegate: SliverChildBuilderDelegate((context, index) {
                return Container(
                    width: 20,
                    child: Center(
                      child: CircularProgressIndicator(),
                    ));
              }, childCount: 1),
            )
          ],
        ),
      ),
    );
  }
}
