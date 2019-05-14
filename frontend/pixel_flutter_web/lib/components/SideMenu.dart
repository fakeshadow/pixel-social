import 'package:flutter_web/material.dart';

import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_web/widgets.dart';

import 'package:pixel_flutter_web/blocs/CategoryBlocs.dart';
import 'package:pixel_flutter_web/blocs/ErrorBlocs.dart';

import '../env.dart';

class SideMenu extends StatelessWidget with env {
  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(left: 30.0),
      child: Container(
        width: 200,
        child: ListView(
          children: <Widget>[
            SizedBox(
              height: 20,
            ),
            Container(
                decoration: BoxDecoration(
                    border: Border.all(
                      color: Colors.black,
                      width: 1.0,
                    ),
                    boxShadow: [
                      new BoxShadow(
                        color: Colors.black12,
                        blurRadius: 2.0,
                      ),
                    ],
                    shape: BoxShape.rectangle),
                child: PopularCategory()),
            SizedBox(
              height: 20,
            ),
            Container(
              width: 200,
              color: Colors.black,
              height: 200,
            ),
            SizedBox(
              height: 20,
            ),
            Container(
              width: 200,
              color: Colors.amber,
              height: 200,
            ),
            SizedBox(
              height: 20,
            ),
            Container(
              width: 200,
              color: Colors.blue,
              height: 200,
            ),
          ],
        ),
      ),
    );
  }
}

class PopularCategory extends StatelessWidget with env {
  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Container(
          padding: const EdgeInsets.only(left: 10.0),
          alignment: Alignment.centerLeft,
          height: 20,
          child: Text('Popular Node'),
        ),
        BlocBuilder(
          bloc: BlocProvider.of<CategoryBloc>(context),
          builder: (context, state) {
            if (state is CategoryFailed) {
              BlocProvider.of<ErrorBloc>(context)
                  .dispatch(GetError(error: "Failed to load categories"));
              return Container();
            } else if (state is CategoryInit || state is CategoryLoading) {
              return Container(child: CircularProgressIndicator());
            } else if (state is CategoryLoaded) {
              return ListView.builder(
                physics: ClampingScrollPhysics(),
                shrinkWrap: true,
                itemCount: 5,
                itemBuilder: (context, index) {
                  return InkWell(
                    onTap: () {
                      // ToDo: Add hero widget and push router to category page
                      print("test");
                    },
                    child: Stack(
                      children: <Widget>[
                        Image.network(
                            url + 'public/${state.categories[index].thumbnail}',
                            width: 200,
                            height: 40,
                            fit: BoxFit.fitWidth),
                        Container(
                          width: 200,
                          height: 40,
                          color: Colors.black26.withOpacity(0.4),
                        ),
                        Positioned(
                          top: 10,
                          left: 30,
                          child: Text(
                            state.categories[index].name,
                            style: TextStyle(color: Colors.white, fontSize: 16),
                          ),
                        ),
                      ],
                    ),
                  );
                },
              );
            } else {
              return Container();
            }
          },
        ),
        Container(
            padding: const EdgeInsets.only(right: 10),
            alignment: Alignment.centerRight,
            height: 25,
            child: InkWell(
              // ToDo: Hero widget and push to categories page.
              onTap: () {},
              child: Text('...Load more',style: TextStyle(fontSize: 15)),
            )),
      ],
    );
  }
}
