package com.httparena;

import io.helidon.webserver.grpc.GrpcService;

import benchmark.Benchmark;
import com.google.protobuf.Descriptors;
import io.grpc.stub.StreamObserver;

import static io.helidon.grpc.core.ResponseHelper.complete;

final class BenchmarkGrpcService implements GrpcService {
    @Override
    public String serviceName() {
        return "BenchmarkService";
    }

    @Override
    public Descriptors.FileDescriptor proto() {
        return Benchmark.getDescriptor();
    }

    @Override
    public void update(Routing router) {
        router.unary("GetSum", this::getSum);
    }

    private void getSum(Benchmark.SumRequest request, StreamObserver<Benchmark.SumReply> observer) {
        complete(observer, Benchmark.SumReply.newBuilder()
                .setResult(request.getA() + request.getB())
                .build());
    }
}
